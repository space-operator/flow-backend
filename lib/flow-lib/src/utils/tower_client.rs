use tower::util::BoxCloneSyncService;

pub type TowerClient<T, U, E> = BoxCloneSyncService<T, U, E>;
