use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ed25519_dalek::Signer;
use flow_server::user::*;

fn init_login(m: SignatureAuth, pk: [u8; 32]) {
    m.init_login(0, &pk);
}

fn confirm(m: SignatureAuth, s: &str) {
    let _ = m.confirm(0, s);
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let sk = ed25519_dalek::SecretKey::from_bytes(&rand::random::<[u8; 32]>()).unwrap();
    let kp = ed25519_dalek::Keypair {
        public: (&sk).into(),
        secret: sk,
    };
    let m = SignatureAuth::new(rand::random());
    let pk = *kp.public.as_bytes();
    c.bench_function("init_login", |b| {
        b.iter(|| init_login(black_box(m), black_box(pk)))
    });

    let msg = m.init_login(0, &pk);
    let signature = bs58::encode(kp.sign(msg.as_bytes())).into_string();
    let text = format!("{msg}.{signature}");
    let s = text.as_str();
    c.bench_function("confirm", |b| {
        b.iter(|| confirm(black_box(m), black_box(s)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
