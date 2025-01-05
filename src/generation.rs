use super::{
    BigUint, CryptoRngCore, Integer, IntoBigUint, ModInverse, RandBigInt, RandPrime, RsaPrivateKey,
};

pub struct MyAwesomeRsaGenerator;
impl MyAwesomeRsaGenerator {
    pub fn new<R>(rng: &mut R, bits: usize) -> Result<RsaPrivateKey, rsa::Error>
    where
        R: CryptoRngCore + ?Sized,
    {
        let p = rng.gen_prime(bits);
        let q = rng.gen_prime(bits);

        let n = &p * &q;
        let gcd = p.gcd(&q);

        let lambda = (&p - BigUint::from(1u32)) * (&q - BigUint::from(1u32)) / gcd;

        let d: BigUint;
        let e: BigUint;
        let mut attempt = 1;

        // Initialize e and d
        'init_e_d: loop {
            attempt += 1;
            println!("Attempt {}", attempt);
            let test_d = rng.gen_biguint(bits / 4);

            if test_d.gcd(&lambda) == BigUint::from(1u32)
                && test_d <= (n.clone() / BigUint::from(18u32)).nth_root(4)
            {
                let Some(test_e_int) = test_d.clone().mod_inverse(&lambda) else {
                    continue 'init_e_d;
                };
                let Some(test_e) = test_e_int.into_biguint() else {
                    continue 'init_e_d;
                };

                d = test_d;
                e = test_e;
                break;
            }
        }

        RsaPrivateKey::from_components(n, e, d, vec![p, q])
    }
}
