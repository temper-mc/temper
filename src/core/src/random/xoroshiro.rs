use crate::random::{
    seed_from_hash, seed_from_pos, u128_high, u128_low, u64_to_u128, upgrade_seed_mixed,
    PositionalRandom, RandomSource,
};
use std::borrow::Borrow;
use std::ops::Not;

pub struct XoroshiroRandomSource {
    seed: u128,
}

pub struct XoroshiroPositionalRandom {
    seed: u128,
}

impl XoroshiroRandomSource {
    const F32_UNIT: f32 = 5.9604645e-8f32;
    const F64_UNIT: f32 = 1.110223e-16f32;

    pub fn new(seed: u64) -> XoroshiroRandomSource {
        let seed = upgrade_seed_mixed(seed);
        XoroshiroRandomSource::new_parts(u128_low(seed), u128_high(seed))
    }

    pub fn new_parts(low: u64, high: u64) -> XoroshiroRandomSource {
        if (low | high) == 0 {
            Self::new_u128(0x6a09e667f3bcc9099e3779b97f4a7c15)
        } else {
            Self::new_u128(u64_to_u128(low, high))
        }
    }

    fn new_u128(seed: u128) -> XoroshiroRandomSource {
        XoroshiroRandomSource { seed }
    }

    fn next_bits(&mut self, count: u8) -> u64 {
        debug_assert!(count <= 64);
        self.next_u64() >> (64 - count)
    }
}

impl XoroshiroPositionalRandom {
    fn new(seed_low: u64, seed_high: u64) -> XoroshiroPositionalRandom {
        XoroshiroPositionalRandom {
            seed: u64_to_u128(seed_low, seed_high),
        }
    }
}

impl RandomSource for XoroshiroRandomSource {
    fn fork(&mut self) -> Self
    where
        Self: Sized,
    {
        XoroshiroRandomSource::new_parts(self.next_u64(), self.next_u64())
    }

    fn fork_positional(&mut self) -> impl PositionalRandom<Self>
    where
        Self: Sized,
    {
        XoroshiroPositionalRandom::new(self.next_u64(), self.next_u64())
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u32_bounded(&mut self, limit: u32) -> u32 {
        let mut random_bits = self.next_u64() & u64::from(u32::MAX);
        let mut multiplied_random_bits = random_bits.wrapping_mul(u64::from(limit));
        let mut fractional_part = multiplied_random_bits as u32;

        if fractional_part < limit {
            let unbiased_buckets_start_idx = limit.not().wrapping_add(1) % limit;
            while fractional_part < unbiased_buckets_start_idx {
                random_bits = self.next_u64() & u64::from(u32::MAX);
                multiplied_random_bits = random_bits.wrapping_mul(u64::from(limit));
                fractional_part = multiplied_random_bits as u32;
            }
        }

        (multiplied_random_bits >> 32) as u32
    }

    fn next_u64(&mut self) -> u64 {
        let s0 = u128_low(self.seed);
        let s1 = u128_high(self.seed);
        let res = s0.wrapping_add(s1).rotate_left(17).wrapping_add(s0);
        let s1 = s1 ^ s0;
        let low = s0.rotate_left(49) ^ s1 ^ s1 << 21;
        let high = s1.rotate_left(28);
        self.seed = u64_to_u128(low, high);
        res
    }

    fn next_bool(&mut self) -> bool {
        (self.next_u64() & 1u64) != 0u64
    }

    fn next_f32(&mut self) -> f32 {
        self.next_bits(24) as f32 * Self::F32_UNIT
    }

    fn next_f64(&mut self) -> f64 {
        f64::from(self.next_bits(53) as f32 * Self::F64_UNIT)
    }

    fn consume_count(&mut self, count: usize) {
        for _ in 0..count {
            let _ = self.next_u64();
        }
    }
}

impl PositionalRandom<XoroshiroRandomSource> for XoroshiroPositionalRandom {
    fn spawn_at(&self, x: i32, y: i32, z: i32) -> XoroshiroRandomSource {
        let pos_seed = seed_from_pos(x, y, z);
        XoroshiroRandomSource::new_parts(pos_seed ^ u128_low(self.seed), u128_high(self.seed))
    }

    fn spawn_from_seed(&self, seed: u64) -> XoroshiroRandomSource {
        let seed = u64_to_u128(seed, seed);
        XoroshiroRandomSource::new_u128(seed ^ self.seed)
    }

    fn spawn_from_hash<S: Borrow<str>>(&self, hash: S) -> XoroshiroRandomSource {
        let seed = seed_from_hash(hash.borrow());
        XoroshiroRandomSource::new_u128(seed ^ self.seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    type XoroshiroTest<'a, T> = &'a [(u64, &'a [T])];

    const U64_TESTS: XoroshiroTest<u64> = &[
        (
            0x4034b697067b413c,
            &[
                0xa55d9b5f163fadb2,
                0xfbf2c8703fdf1dc3,
                0x517befdf268a71a2,
                0xcbe0d917cfa064d,
                0x635089091e19ff2,
                0xdd5904e59d436ab,
                0x365668e0f6cbb899,
                0x662454bc5f976c2b,
                0x65f2e9a1df230257,
                0xfcbedacc0bffd474,
                0x863ee5c9d7a5948e,
                0xff03a48ec1dd1d18,
                0x190788313bb310ac,
                0xf17b45909a5c1e56,
                0x439a1030163afc42,
            ],
        ),
        (
            0xf5ef27db8b20f637,
            &[
                0xc37c9fa466b8f4cc,
                0x847b48a321febde,
                0x9145e5625087ab0f,
                0xcee6c2359471c191,
                0x41365be8316fbc33,
                0x13e1107979e5481b,
                0xbd67e86f731d601a,
                0xf82a2f09312c496,
                0x1810852a3ca99088,
                0xe2ad62725a4123d4,
                0xda4d889d261bd831,
                0xa8f60d4e1cbc7457,
                0xe9b3613dc2a8f4ea,
                0xe2bec28b1011a82e,
                0x8868f1443ce78992,
            ],
        ),
        (
            0x8e6b3081605e1ea0,
            &[
                0x519a6f8daff8944e,
                0x357d1d7fc1fd81d6,
                0xc52530c170331b4,
                0x345fba24a0ee09ab,
                0x25f1ad4a07cd8f67,
                0x3bc92d975bd75eac,
                0x1605deb993980900,
                0x386914cb07985dc4,
                0xb95476ad291f1bf4,
                0x861640b892748666,
                0x94a9d1bf0c387343,
                0xe871b3c6f17c64a9,
                0x7339f6dc93553ca7,
                0x4e2b79055e68a97f,
                0x484a2c01994ff107,
            ],
        ),
        (
            0xeab51b1de0dcc6b9,
            &[
                0xa836c7247064a816,
                0x44481186bbfa0c6e,
                0xd2dc6ffa73c672f4,
                0xf81c40cca5bd01b2,
                0xfd4a772c4f2e4a8b,
                0x9f88fdd98f07846,
                0x58fb4ce6e4f0cfd0,
                0x3080ad5c80abdf10,
                0xf8d835ffc8c3075f,
                0x916a7178be2245c7,
                0xa82a39dee0fb45ca,
                0xd8e8511225a001e2,
                0xffba4fac5eff1ea5,
                0x391f8a7271e916f,
                0xc67fbf7c50814709,
            ],
        ),
        (
            0x74c7aec6b782abd5,
            &[
                0x7083fc188361b8b9,
                0x8e2b272b86c58b77,
                0x3266b6f69bb20ed6,
                0x67b03abcb24a71c,
                0x48ba82156522bd08,
                0xe9cd85f544cd2218,
                0x2a6619d16c75a421,
                0x40ecd548980b0418,
                0xea0cb22d94758545,
                0x7bedd29a05092710,
                0x487aa3370af4e268,
                0x7d2cd3ed2cfacfad,
                0x65e5ec5af67569fc,
                0xdf2f71e2b85834d0,
                0xb9755848eec129ce,
            ],
        ),
        (
            0xef0c4405d7baae04,
            &[
                0x829180b4ae1890a0,
                0x7ee3995f68f20cfc,
                0x1925139d250bfb23,
                0x7a6c565245c13794,
                0xad7f470f2e3d826a,
                0x9d1bdc54b77848df,
                0xa98ed3228d234668,
                0x6b9b377208bf2a1d,
                0x3dc0df4debd34f10,
                0xb86ce9b6cc4d9586,
                0xa21656861f2849ee,
                0x5e7a93a81584bdc7,
                0x78bd2751856a574e,
                0x8f013d8fbe931277,
                0x3c84ccd4dcf0103d,
            ],
        ),
        (
            0x54148e1e8f772e8f,
            &[
                0x45571ed1cc8a191a,
                0x5002141e9f5ee7ce,
                0xefdd2203c6b0debb,
                0x59e7d7de9c75c42b,
                0x890e9293de5f91cd,
                0x537125a77a75532b,
                0x6495dbdf068f1fdf,
                0x3b119ed22ec0f50a,
                0x9523b0ac7a95f44d,
                0x79610e1d35452de6,
                0x8f123eb9a7949e68,
                0x6d30b7a0a8c4a8f9,
                0xa950039e5880f38b,
                0xeb6e7a4f145bb852,
                0x64b1cd147117e9bf,
            ],
        ),
        (
            0xca96d036332fee20,
            &[
                0xef89e600d048056b,
                0x43f78fa5a2d27d81,
                0xacadc5d11696754b,
                0xb8b2d5c1666bcd0e,
                0x60ae526c2aa9e2d5,
                0xa47995cdaf932422,
                0x33886615d0c32151,
                0xdb04f37ac31f67d1,
                0x1964d74be1861a2f,
                0xc856b274e36c03dc,
                0xf99333345b3fa284,
                0x2c52368a7100e801,
                0x44c7236b8ddd8a3a,
                0xf2846b1931221a0e,
                0xa9b3b28561d8c081,
            ],
        ),
        (
            0xbe610162705e9d7f,
            &[
                0x36b7cd781017c50c,
                0x770a597926569dfa,
                0x252b39863d576d8d,
                0x8668b8ec5609fb0e,
                0xd825583eef282f33,
                0x19908d014d4ce8dc,
                0x43125f1ae3168e47,
                0x624168ce15d39ec1,
                0x3f39023c50c3e47,
                0xe4a2f3ca99695138,
                0x36c85fb257ddd8b4,
                0xf571442d9c8fefdc,
                0xc9f5debb62c05b13,
                0xc5854edcacbfb7e1,
                0xc8c0dfc6d8d705c0,
            ],
        ),
        (
            0xf8cffc6917849d2,
            &[
                0x598c76eaef059b98,
                0x6d3eaf5d117ccb2d,
                0x8402e5f78a035f29,
                0xfad0d38b46c03e09,
                0x1f989aa2c7ec3b43,
                0x619054d5f2112b46,
                0xc3dbfbe2f5c30d6e,
                0xcd5b38660c0059d3,
                0xb76ae64a311797ec,
                0xbdda360facd63580,
                0x82a173a2d7a74746,
                0xcd4d6ba4c9c7a946,
                0xf0ede1e98abbf5b6,
                0x42f123a14dab106a,
                0x85af17634d1887e1,
            ],
        ),
    ];

    const F64_TESTS: XoroshiroTest<f64> = &[
        (
            0x4034b697067b413c,
            &[
                0.6459595561027527,
                0.9841732978820801,
                0.3182973861694336,
                0.049774978309869766,
                0.02424672618508339,
                0.05403997376561165,
                0.21225601434707642,
                0.3989918529987335,
                0.39823779463768005,
                0.9872872233390808,
                0.5243972539901733,
                0.9961493611335754,
                0.09777118265628815,
                0.9432872533798218,
                0.2640695571899414,
            ],
        ),
        (
            0xf5ef27db8b20f637,
            &[
                0.7636203765869141,
                0.032344136387109756,
                0.5674727559089661,
                0.8082085847854614,
                0.25473570823669434,
                0.0776529610157013,
                0.7398667335510254,
                0.06058710440993309,
                0.09400207549333572,
                0.8854581117630005,
                0.8527455925941467,
                0.6600044369697571,
                0.9128933548927307,
                0.8857232928276062,
                0.5328512787818909,
            ],
        ),
        (
            0x8e6b3081605e1ea0,
            &[
                0.31876274943351746,
                0.20894035696983337,
                0.048131171613931656,
                0.20458568632602692,
                0.14821894466876984,
                0.23353847861289978,
                0.08602707087993622,
                0.2203534096479416,
                0.7239450812339783,
                0.5237770676612854,
                0.5807162523269653,
                0.9079849720001221,
                0.450103223323822,
                0.3053508400917053,
                0.2823817729949951,
            ],
        ),
        (
            0xeab51b1de0dcc6b9,
            &[
                0.6570858359336853,
                0.26672467589378357,
                0.8236761093139648,
                0.9691811203956604,
                0.9894174933433533,
                0.03894900530576706,
                0.3475845456123352,
                0.18946345150470734,
                0.9720491170883179,
                0.5680304169654846,
                0.656894326210022,
                0.8472948670387268,
                0.998936653137207,
                0.01394609548151493,
                0.7753867506980896,
            ],
        ),
        (
            0x74c7aec6b782abd5,
            &[
                0.4395139217376709,
                0.5553459525108337,
                0.1968798041343689,
                0.02531454898416996,
                0.2840958833694458,
                0.9132922887802124,
                0.16562043130397797,
                0.25361379981040955,
                0.91425621509552,
                0.4840976297855377,
                0.2831212878227234,
                0.4889652729034424,
                0.3980396091938019,
                0.8718177080154419,
                0.7244467735290527,
            ],
        ),
        (
            0xef0c4405d7baae04,
            &[
                0.5100327134132385,
                0.4956603944301605,
                0.09822199493646622,
                0.47821560502052307,
                0.6777233481407166,
                0.6137063503265381,
                0.6623355746269226,
                0.42033717036247253,
                0.2412242442369461,
                0.7204118967056274,
                0.6331533789634705,
                0.36905786395072937,
                0.47163626551628113,
                0.5586127042770386,
                0.23640136420726776,
            ],
        ),
        (
            0x54148e1e8f772e8f,
            &[
                0.27086061239242554,
                0.3125317096710205,
                0.9369679689407349,
                0.351193904876709,
                0.535378634929657,
                0.3259452283382416,
                0.3929116725921631,
                0.2307376116514206,
                0.5825758576393127,
                0.47413718700408936,
                0.5588721632957458,
                0.4265246093273163,
                0.6613771915435791,
                0.9196544885635376,
                0.39333802461624146,
            ],
        ),
        (
            0xca96d036332fee20,
            &[
                0.9356979131698608,
                0.26549622416496277,
                0.674526572227478,
                0.7214788198471069,
                0.37765994668006897,
                0.6424802541732788,
                0.2013000249862671,
                0.8555442690849304,
                0.0991949588060379,
                0.782572865486145,
                0.974902331829071,
                0.17312946915626526,
                0.268663614988327,
                0.947333037853241,
                0.6628982424736023,
            ],
        ),
        (
            0xbe610162705e9d7f,
            &[
                0.21374210715293884,
                0.4650016725063324,
                0.14519080519676208,
                0.5250354409217834,
                0.8443198204040527,
                0.09986191987991333,
                0.26199907064437866,
                0.3838105797767639,
                0.015435227192938328,
                0.8931114673614502,
                0.21399496495723724,
                0.9587595462799072,
                0.7889079451560974,
                0.7715653777122498,
                0.7841930389404297,
            ],
        ),
        (
            0xf8cffc6917849d2,
            &[
                0.34979957342147827,
                0.4267377555370331,
                0.5156692266464233,
                0.9797489643096924,
                0.1234223023056984,
                0.3811085820198059,
                0.7650754451751709,
                0.8021731376647949,
                0.7164748907089233,
                0.7416108846664429,
                0.5102760791778564,
                0.8019626140594482,
                0.9411298036575317,
                0.261491984128952,
                0.5222029089927673,
            ],
        ),
    ];

    const U32_BOUND_TESTS: XoroshiroTest<u32> = &[
        (
            0x4034b697067b413c,
            &[1, 3, 2, 7, 8, 5, 14, 5, 13, 0, 12, 11, 3, 9, 1],
        ),
        (
            0xf5ef27db8b20f637,
            &[6, 2, 4, 8, 2, 7, 6, 8, 3, 5, 2, 1, 11, 0, 3],
        ),
        (
            0x8e6b3081605e1ea0,
            &[10, 11, 1, 9, 0, 5, 8, 0, 2, 8, 0, 14, 8, 5, 8],
        ),
        (
            0xeab51b1de0dcc6b9,
            &[6, 11, 6, 9, 4, 8, 13, 7, 11, 11, 13, 2, 5, 2, 4],
        ),
        (
            0x74c7aec6b782abd5,
            &[7, 7, 9, 11, 5, 4, 6, 8, 8, 0, 0, 2, 14, 10, 13],
        ),
        (
            0xef0c4405d7baae04,
            &[10, 6, 2, 4, 2, 10, 8, 0, 13, 11, 1, 1, 7, 11, 12],
        ),
        (
            0x54148e1e8f772e8f,
            &[11, 9, 11, 9, 13, 7, 0, 2, 7, 3, 9, 9, 5, 1, 6],
        ),
        (
            0xca96d036332fee20,
            &[12, 9, 1, 6, 2, 10, 12, 11, 13, 13, 5, 6, 8, 2, 5],
        ),
        (
            0xbe610162705e9d7f,
            &[0, 2, 3, 5, 14, 4, 13, 1, 11, 8, 5, 9, 5, 10, 12],
        ),
        (
            0xf8cffc6917849d2,
            &[14, 1, 8, 4, 11, 14, 14, 0, 2, 10, 12, 11, 8, 4, 4],
        ),
    ];

    fn run_test<T: PartialEq + Debug>(
        test: &XoroshiroTest<T>,
        supplier: fn(&mut XoroshiroRandomSource) -> T,
    ) {
        for (seed_num, (seed, values)) in test.iter().enumerate() {
            let mut random = XoroshiroRandomSource::new(*seed);

            for (i, value) in values.iter().enumerate() {
                assert_eq!(
                    supplier(&mut random),
                    *value,
                    "Value mismatch. Seed #{} and Number #{}",
                    seed_num,
                    i
                );
            }
        }
    }

    #[test]
    fn test_next_u64() {
        run_test(&U64_TESTS, XoroshiroRandomSource::next_u64);
    }

    #[test]
    fn test_next_f64() {
        run_test(&F64_TESTS, XoroshiroRandomSource::next_f64);
    }

    #[test]
    fn test_next_u32_bounded() {
        run_test(&U32_BOUND_TESTS, |rand| rand.next_u32_bounded(15))
    }
}
