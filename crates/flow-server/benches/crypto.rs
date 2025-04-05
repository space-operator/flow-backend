use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ed25519_dalek::Signer;
use flow_server::user::*;

fn init_login(m: SignatureAuth, pk: [u8; 32]) {
    m.init_login(0, &pk);
}

fn confirm(m: SignatureAuth, s: &str) {
    let _ = m.confirm(0, s);
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let sk = rand::random::<[u8; 32]>();
    let kp = ed25519_dalek::SigningKey::from_bytes(&sk);
    let m = SignatureAuth::new(rand::random());
    let pk = *kp.verifying_key().as_bytes();
    c.bench_function("init_login", |b| {
        b.iter(|| init_login(black_box(m), black_box(pk)))
    });

    let msg = m.init_login(0, &pk);
    let signature = bs58::encode(kp.sign(msg.as_bytes()).to_bytes()).into_string();
    let text = format!("{msg}.{signature}");
    let s = text.as_str();
    c.bench_function("confirm", |b| {
        b.iter(|| confirm(black_box(m), black_box(s)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
