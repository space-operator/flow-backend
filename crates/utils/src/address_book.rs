use actix::{Actor, ArbiterHandle, WeakAddr};
use hashbrown::HashMap;
use std::{
    any::Any,
    borrow::Borrow,
    hash::{Hash, Hasher},
};

#[derive(Default)]
pub struct AddressBook {
    addrs: HashMap<AnyID, Box<dyn Any>>,
}

impl AddressBook {
    pub fn new() -> Self {
        Self {
            addrs: HashMap::new(),
        }
    }

    pub fn get_or_start<A, F>(&mut self, id: A::ID, start: F) -> actix::Addr<A>
    where
        A: Actor<Context = actix::Context<A>>,
        A: ManagableActor,
        F: FnOnce() -> actix::Addr<A>,
    {
        self.get::<A>(id.clone())
            .and_then(|weak| weak.upgrade())
            .unwrap_or_else(move || {
                let addr = start();
                self.addrs
                    .insert(AnyID::new::<A>(id), Box::new(addr.downgrade()));
                addr
            })
    }

    pub fn start<A>(&mut self, actor: A) -> actix::Addr<A>
    where
        A: Actor<Context = actix::Context<A>>,
        A: ManagableActor,
    {
        let id = actor.id();
        let addr = actor.start();
        let weak = addr.downgrade();
        self.addrs.insert(AnyID::new::<A>(id), Box::new(weak));
        addr
    }

    pub fn try_start_in_rt<A>(&mut self, actor: A, rt: ArbiterHandle) -> Result<actix::Addr<A>, A>
    where
        A: Actor<Context = actix::Context<A>>,
        A: ManagableActor + Send,
    {
        let id = actor.id();
        if let hashbrown::hash_map::Entry::Vacant(slot) = self.addrs.entry(AnyID::new::<A>(id)) {
            let addr = A::start_in_arbiter(&rt, move |_| actor);
            slot.insert(Box::new(addr.downgrade()));
            Ok(addr)
        } else {
            Err(actor)
        }
    }

    pub fn try_start_with_context<A, F>(
        &mut self,
        id: A::ID,
        make_actor: F,
        rt: ArbiterHandle,
    ) -> Result<actix::Addr<A>, ()>
    where
        A: Actor<Context = actix::Context<A>>,
        A: ManagableActor + Send,
        F: FnOnce(&mut actix::Context<A>) -> A + Send + 'static,
    {
        if let hashbrown::hash_map::Entry::Vacant(slot) = self.addrs.entry(AnyID::new::<A>(id)) {
            let addr = A::start_in_arbiter(&rt, make_actor);
            slot.insert(Box::new(addr.downgrade()));
            Ok(addr)
        } else {
            Err(())
        }
    }

    pub fn get<A>(&self, id: A::ID) -> Option<WeakAddr<A>>
    where
        A: ManagableActor,
    {
        self.addrs
            .get(&ID::<A> { id } as &dyn HashKey)
            .map(|boxed| boxed.downcast_ref::<WeakAddr<A>>().unwrap().clone())
    }

    #[must_use]
    pub fn insert<A>(&mut self, id: A::ID, addr: WeakAddr<A>) -> bool
    where
        A: ManagableActor,
    {
        if let hashbrown::hash_map::Entry::Vacant(slot) = self.addrs.entry(AnyID::new::<A>(id)) {
            slot.insert(Box::new(addr));
            true
        } else {
            false
        }
    }

    pub fn try_insert<A>(
        &mut self,
        id: A::ID,
        addr: WeakAddr<A>,
    ) -> Result<(), (A::ID, WeakAddr<A>)>
    where
        A: ManagableActor,
    {
        if let hashbrown::hash_map::Entry::Vacant(slot) =
            self.addrs.entry(AnyID::new::<A>(id.clone()))
        {
            slot.insert(Box::new(addr));
            Ok(())
        } else {
            Err((id, addr))
        }
    }

    pub fn iter<'a, A: ManagableActor>(
        &'a self,
    ) -> impl Iterator<Item = (A::ID, actix::Addr<A>)> + 'a {
        self.addrs.iter().filter_map(|(k, v)| {
            v.downcast_ref::<WeakAddr<A>>()
                .and_then(|weak| weak.upgrade())
                .and_then(|addr| {
                    k.id.as_any()
                        .downcast_ref::<ID<A>>()
                        .map(|id| (id.id.clone(), addr))
                })
        })
    }
}

impl Borrow<dyn HashKey> for AnyID {
    fn borrow(&self) -> &dyn HashKey {
        self.id.as_ref()
    }
}

pub trait ManagableActor: Any + Actor {
    type ID: Hash + Eq + Clone;

    fn id(&self) -> Self::ID;
}

struct ID<A>
where
    A: ManagableActor,
{
    id: A::ID,
}

impl<A> Clone for ID<A>
where
    A: ManagableActor,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
        }
    }
}

trait HashKey: Any {
    fn as_any(&self) -> &dyn Any;
    fn dyn_eq(&self, other: &dyn HashKey) -> bool;
    fn clone_box(&self) -> Box<dyn HashKey>;
    fn dyn_hash(&self, state: &mut dyn Hasher);
}

impl Hash for dyn HashKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state as &mut dyn Hasher);
    }
}

impl PartialEq for dyn HashKey {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other)
    }
}

impl Eq for dyn HashKey {}

impl<A: ManagableActor> HashKey for ID<A> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn dyn_eq(&self, other: &dyn HashKey) -> bool {
        match other.as_any().downcast_ref::<ID<A>>() {
            Some(other) => other.id == self.id,
            None => false,
        }
    }
    fn clone_box(&self) -> Box<dyn HashKey> {
        Box::new(self.clone())
    }
    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        self.type_id().hash(&mut state);
        self.id.hash(&mut state);
    }
}

struct AnyID {
    id: Box<dyn HashKey>,
}

impl Clone for AnyID {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone_box(),
        }
    }
}

impl Hash for AnyID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.dyn_hash(state);
    }
}

impl Eq for AnyID {}

impl PartialEq for AnyID {
    fn eq(&self, other: &Self) -> bool {
        self.id.dyn_eq(other.id.as_ref())
    }
}

impl AnyID {
    fn new<A: ManagableActor>(id: A::ID) -> Self {
        AnyID {
            id: Box::new(ID::<A> { id }),
        }
    }
}
