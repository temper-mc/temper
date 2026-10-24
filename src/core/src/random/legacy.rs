use crate::math::java_hash_code;
use crate::random::{PositionalRandom, RandomSource};
use std::borrow::Borrow;

pub struct LegacyRandomSource {
    seed: u64,
}

pub struct LegacyPositionalRandom {
    seed: u64,
}

impl LegacyRandomSource {
    pub fn new(seed: u64) -> Self {
        Self {
            seed: (seed ^ 25214903917) & 281474976710655,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(25214903917).wrapping_add(11) & 281474976710655;
        (self.seed >> (48 - bits)) as u32
    }
}

impl RandomSource for LegacyRandomSource {
    fn fork(&mut self) -> Self
    where
        Self: Sized,
    {
        Self::new(self.next_u64())
    }

    fn fork_positional(&mut self) -> impl PositionalRandom<Self>
    where
        Self: Sized,
    {
        LegacyPositionalRandom {
            seed: self.next_u64(),
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.next_bits(32)
    }

    fn next_u32_bounded(&mut self, limit: u32) -> u32 {
        if (limit & (limit - 1)) == 0 {
            return (u64::from(limit).wrapping_mul(u64::from(self.next_bits(31))) >> 31) as u32;
        }

        let mut sample = self.next_bits(31);
        let mut modulo = (sample as i32 % limit as i32) as u32;
        while sample.wrapping_add(limit - 1) < modulo {
            sample = self.next_bits(31);
            modulo = (sample as i32 % limit as i32) as u32;
        }

        modulo
    }

    fn next_u64(&mut self) -> u64 {
        let upper = u64::from(self.next_u32());
        let lower = i64::from(self.next_u32() as i32);
        (upper << 32).wrapping_add_signed(lower)
    }

    fn next_bool(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    fn next_f32(&mut self) -> f32 {
        self.next_bits(24) as f32 * 5.9604645E-8
    }

    fn next_f64(&mut self) -> f64 {
        let upper = u64::from(self.next_bits(26));
        let lower = i64::from(self.next_bits(27) as i32);
        let combined = (upper << 27).wrapping_add_signed(lower) as f32;
        f64::from(combined * 1.110223E-16)
    }

    fn consume_count(&mut self, count: usize) {
        for _ in 0..count {
            self.next_u32();
        }
    }
}

impl PositionalRandom<LegacyRandomSource> for LegacyPositionalRandom {
    fn spawn_at(&self, x: i32, y: i32, z: i32) -> LegacyRandomSource {
        let positional_seed = get_seed(x, y, z);
        LegacyRandomSource::new(positional_seed ^ self.seed)
    }

    fn spawn_from_seed(&self, seed: u64) -> LegacyRandomSource {
        LegacyRandomSource::new(seed)
    }

    fn spawn_from_hash<S: Borrow<str>>(&self, hash: S) -> LegacyRandomSource {
        let positional_seed = u64::from(java_hash_code(hash.borrow()));
        LegacyRandomSource::new(positional_seed ^ self.seed)
    }
}

fn get_seed(x: i32, y: i32, z: i32) -> u64 {
    let seed = (x as u64).wrapping_mul(3129871) ^ (z as u64).wrapping_mul(116129781) ^ y as u64;
    let seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42317861)
        .wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}

#[cfg(test)]
mod tests {
    use super::*;

    type LegacyTests<'a, T> = &'a [(u64, &'a [T])];

    const F64_TESTS: LegacyTests<f64> = &[
        (
            0x2a2ca488f66f517e,
            &[
                0.03147638589143753,
                0.7190093398094177,
                0.6706510186195374,
                0.4290236830711365,
                0.0986199676990509,
                0.6238958239555359,
                0.08519502729177475,
                0.8366659283638,
                0.09068159013986588,
                0.528780460357666,
                0.33723893761634827,
                0.13068076968193054,
                0.8269587159156799,
                0.4299091398715973,
                0.5878450870513916,
            ],
        ),
        (
            0xccbc22d72e97c372,
            &[
                0.5515309572219849,
                0.18569910526275635,
                0.3135964274406433,
                0.18837898969650269,
                0.37393295764923096,
                0.22070811688899994,
                0.14057563245296478,
                0.20478256046772003,
                0.4953804612159729,
                0.7870016694068909,
                0.14163966476917267,
                0.7544727921485901,
                0.578295111656189,
                0.17887690663337708,
                0.3517719507217407,
            ],
        ),
        (
            0x404e64b826f4b9f4,
            &[
                0.28571078181266785,
                0.8644270896911621,
                0.8925256729125977,
                0.42938241362571716,
                0.6989474892616272,
                0.7335751056671143,
                0.389543354511261,
                0.7895180583000183,
                0.9392277598381042,
                0.07538992911577225,
                0.6903610229492188,
                0.4453928768634796,
                0.009527619928121567,
                0.4816451668739319,
                0.9772804379463196,
            ],
        ),
        (
            0x1dfbe5a84fb8f31b,
            &[
                0.04258017987012863,
                0.5345064401626587,
                0.09610435366630554,
                0.29182004928588867,
                0.6263471245765686,
                0.08274491876363754,
                0.6116940975189209,
                0.9371387362480164,
                0.42140159010887146,
                0.6410727500915527,
                0.43161749839782715,
                0.8368074893951416,
                0.9127132892608643,
                0.13565181195735931,
                0.9676677584648132,
            ],
        ),
        (
            0x1986c1cae1d8f2c5,
            &[
                0.5516881346702576,
                0.053172748535871506,
                0.9106011986732483,
                0.38479137420654297,
                0.25562533736228943,
                0.9368635416030884,
                0.07317247241735458,
                0.6098326444625854,
                0.6299704313278198,
                0.1338902860879898,
                0.8952486515045166,
                0.5827803611755371,
                0.13936425745487213,
                0.3194708526134491,
                0.9285292029380798,
            ],
        ),
        (
            0xc1b5c37dfd2e8a4f,
            &[
                0.8435527086257935,
                0.5218474268913269,
                0.8216179609298706,
                0.3394281268119812,
                0.8965122103691101,
                0.6270481944084167,
                0.8109346032142639,
                0.6337869167327881,
                0.22692246735095978,
                0.006666144821792841,
                0.9369772672653198,
                0.7508467435836792,
                0.3960116505622864,
                0.6487700939178467,
                0.5591779947280884,
            ],
        ),
        (
            0xc5b7534314657ec2,
            &[
                0.07726091891527176,
                0.19445861876010895,
                0.9423493146896362,
                0.6258829832077026,
                0.9702897071838379,
                0.2345108836889267,
                0.25817760825157166,
                0.05388482287526131,
                0.7821248769760132,
                0.05439234897494316,
                0.31780725717544556,
                0.29989323019981384,
                0.9833123683929443,
                0.3770781457424164,
                0.20406726002693176,
            ],
        ),
        (
            0xf129fd17547d4fe1,
            &[
                0.7829524874687195,
                0.7259644865989685,
                0.8955469131469727,
                0.26045355200767517,
                0.3837604820728302,
                0.5371924042701721,
                0.8908534049987793,
                0.4606555700302124,
                0.5111673474311829,
                0.4479365348815918,
                0.5311049222946167,
                0.9950011968612671,
                0.5235744118690491,
                0.4491877555847168,
                0.0586111918091774,
            ],
        ),
        (
            0x7b061ce6a7d9f7ed,
            &[
                0.7708956599235535,
                0.9307836890220642,
                0.985508382320404,
                0.9810603857040405,
                0.588038980960846,
                0.11680526286363602,
                0.5348349213600159,
                0.5493833422660828,
                0.7739362716674805,
                0.7523193955421448,
                0.22731482982635498,
                0.8296963572502136,
                0.19618092477321625,
                0.07825843244791031,
                0.456740140914917,
            ],
        ),
        (
            0x9c26eea55cfcef8e,
            &[
                0.8124504685401917,
                0.4235464930534363,
                0.43931064009666443,
                0.3611891567707062,
                0.058743368834257126,
                0.4208390712738037,
                0.5111594796180725,
                0.16236963868141174,
                0.956344723701477,
                0.7538458108901978,
                0.5922407507896423,
                0.7626545429229736,
                0.2417779266834259,
                0.6499732136726379,
                0.42965835332870483,
            ],
        ),
    ];
    const U64_TESTS: LegacyTests<u64> = &[
        (
            0x2a2ca488f66f517e,
            &[
                0x80ed61393395a7a,
                0xb810ff5771cf51b8,
                0xabafc8d7f3f50f5d,
                0x6dd47eec352c8f09,
                0x193f2848eaedde1d,
                0x9fb7a2ee1c2f8400,
                0x15cf57528237b0d4,
                0xd62fbcd482e7b77c,
                0x1736e887a30118e5,
                0x875e27c88e5429ea,
                0x56554a4778a8f251,
                0x21744b63f8249ab3,
                0xd3b391192b3a6849,
                0x6e0e86bc2b4530f5,
                0x967d045ddccc2f03,
            ],
        ),
        (
            0xccbc22d72e97c372,
            &[
                0x8d31225b9351bdc3,
                0x2f89fa017958e90c,
                0x5047db02218ded3d,
                0x30399af5d6da5f03,
                0x5fba1200f3d3bace,
                0x388053d313868666,
                0x23fcc3e16523333a,
                0x346ca14404a9c99b,
                0x7ed140eff3cf927b,
                0xc978f11f617f0465,
                0x24427f20f31df9d4,
                0xc12520ed27e4fa0d,
                0x940b2630d3d0f325,
                0x2dcae0a920387adf,
                0x5a0dba0a59985581,
            ],
        ),
        (
            0x404e64b826f4b9f4,
            &[
                0x4924574fa057b749,
                0xdd4b17b4f8fa8628,
                0xe47c8ffe82fbc91a,
                0x6dec0196453c3b3c,
                0xb2ee39140efdeec4,
                0xbbcb93f2433a35b1,
                0x63b91d009560d40c,
                0xca1ddb5143063244,
                0xf0713b705079bd78,
                0x134cc12c42420ca3,
                0xb0bb803e87680d04,
                0x7205448b76d43541,
                0x27066cbbe33de10,
                0x7b4d18ef490d3069,
                0xfa2f0d717da5ec97,
            ],
        ),
        (
            0x1dfbe5a84fb8f31b,
            &[
                0xae688dd7b7d7cc1,
                0x88d569caa66cd899,
                0x189a4b940cb21ed6,
                0x4ab4b7f417089dd3,
                0xa058490f9a17deea,
                0x152ec5459a1b2e3a,
                0x9c97fc19f48202fd,
                0xefe8536a9cc9d9fd,
                0x6be0f989c0bdd036,
                0xa41d57fb6c5301aa,
                0x6e7e7bd8f0dbf0e6,
                0xd6390434e4a515d3,
                0xe9a793f4739bb6c6,
                0x22ba13ec3d736ced,
                0xf7b91351a7d1bd83,
            ],
        ),
        (
            0x1986c1cae1d8f2c5,
            &[
                0x8d3b6e9b35c76858,
                0xd9cba80cd359ec4,
                0xe91d28faa392e071,
                0x6281b029434e176d,
                0x4170a96d067e3551,
                0xefd649f6871e3863,
                0x12bb6e41455aa10c,
                0x9c1dfe0c3bc92283,
                0xa145bd9f7862d7dd,
                0x2246a2531ad28edf,
                0xe52f03b9aa79dc49,
                0x953117d588ad3979,
                0x23ad604531407185,
                0x51c8d74cc17eb44d,
                0xedb41725a124d014,
            ],
        ),
        (
            0xc1b5c37dfd2e8a4f,
            &[
                0xd7f31246551d4be5,
                0x8597cae22a5c5da6,
                0xd2558e2b88ca31bf,
                0x56e4c32e2d3f60bd,
                0xe581d368a389f565,
                0xa0863a926cc37165,
                0xcf9968c924a3bf40,
                0xa23fdc1f2735aa9f,
                0x3a17975e462b4065,
                0x1b4df7065361a38,
                0xefddbdd7bc8da561,
                0xc0377e1ee582fb2d,
                0x6561051992b47139,
                0xa615cc79b552b24b,
                0x8f264a09384c84ab,
            ],
        ),
        (
            0xc5b7534314657ec2,
            &[
                0x13c75f168a075434,
                0x31c80a5316598da4,
                0xf13dce2ffc5f56be,
                0xa039ddffd8959877,
                0xf864e7bb7f462273,
                0x3c08e780d30fc88f,
                0x4217ed980beda749,
                0xdcb655d56caa16e,
                0xc83955b7f8b46679,
                0xdeca817d4b4f07d,
                0x515bd1033e7e3999,
                0x4cc5cdb87e5a3c41,
                0xfbba5c7b88e9f6e9,
                0x60883183b3ce72cd,
                0x343dc0a17c4c86ec,
            ],
        ),
        (
            0xf129fd17547d4fe1,
            &[
                0xc86f93685d9b1f0b,
                0xb9d8ce925f3ced6c,
                0xe5428fb74659ab9c,
                0x42ad1554caea0eeb,
                0x623e20b4b938f770,
                0x8985711080362026,
                0xe40ef789f77d78e9,
                0x75ed85def57af7c0,
                0x82dbdd456d443111,
                0x72abf7d45e25f2b4,
                0x87f67e5973838403,
                0xfeb86598bb049d54,
                0x8608f9472f96b389,
                0x72fdf8394ac4688f,
                0xf01249c7e33e006,
            ],
        ),
        (
            0x7b061ce6a7d9f7ed,
            &[
                0xc5596b51307deda1,
                0xee47d739130fb3c9,
                0xfc4a46f1be1871f0,
                0xfb26c5a464dee1d0,
                0x9689b8f489b4e450,
                0x1de6f31a7844af43,
                0x88eaf0c1a2fe02d6,
                0x8ca4635fbf53709d,
                0xc620af92735ed457,
                0xc09800be0b2abee5,
                0x3a314e1f6578ce0a,
                0xd466facf46654feb,
                0x3238e99990242f6b,
                0x1408be9961558ed1,
                0x74ecebe1fabfb861,
            ],
        ),
        (
            0x9c26eea55cfcef8e,
            &[
                0xcffcc0fa3f81de44,
                0x6c6d8b14cee04284,
                0x7076a9b3ebb0002f,
                0x5c76e45f3980699f,
                0xf09ce3ad9516e35,
                0x6bbc1c13123548a5,
                0x82db58f5753bb6f4,
                0x29910e46fcc81303,
                0xf4d3027c2d62b3a4,
                0xc0fc09d3fbd09182,
                0x979d16c78f83106b,
                0xc33d5417dd7a54b9,
                0x3de5288d157c85ef,
                0xa664a4f63a461ac3,
                0x6dfe16e1e41d982d,
            ],
        ),
    ];
    const U32_TESTS: LegacyTests<u32> = &[
        (
            0x2a2ca488f66f517e,
            &[
                0x80ed614, 0x93395a7a, 0xb810ff57, 0x71cf51b8, 0xabafc8d8, 0xf3f50f5d, 0x6dd47eec,
                0x352c8f09, 0x193f2849, 0xeaedde1d, 0x9fb7a2ee, 0x1c2f8400, 0x15cf5753, 0x8237b0d4,
                0xd62fbcd5,
            ],
        ),
        (
            0xccbc22d72e97c372,
            &[
                0x8d31225c, 0x9351bdc3, 0x2f89fa01, 0x7958e90c, 0x5047db02, 0x218ded3d, 0x30399af6,
                0xd6da5f03, 0x5fba1201, 0xf3d3bace, 0x388053d3, 0x13868666, 0x23fcc3e1, 0x6523333a,
                0x346ca144,
            ],
        ),
        (
            0x404e64b826f4b9f4,
            &[
                0x49245750, 0xa057b749, 0xdd4b17b5, 0xf8fa8628, 0xe47c8fff, 0x82fbc91a, 0x6dec0196,
                0x453c3b3c, 0xb2ee3914, 0xefdeec4, 0xbbcb93f2, 0x433a35b1, 0x63b91d01, 0x9560d40c,
                0xca1ddb51,
            ],
        ),
        (
            0x1dfbe5a84fb8f31b,
            &[
                0xae688dd, 0x7b7d7cc1, 0x88d569cb, 0xa66cd899, 0x189a4b94, 0xcb21ed6, 0x4ab4b7f4,
                0x17089dd3, 0xa0584910, 0x9a17deea, 0x152ec546, 0x9a1b2e3a, 0x9c97fc1a, 0xf48202fd,
                0xefe8536b,
            ],
        ),
        (
            0x1986c1cae1d8f2c5,
            &[
                0x8d3b6e9b, 0x35c76858, 0xd9cba81, 0xcd359ec4, 0xe91d28fb, 0xa392e071, 0x6281b029,
                0x434e176d, 0x4170a96d, 0x67e3551, 0xefd649f7, 0x871e3863, 0x12bb6e41, 0x455aa10c,
                0x9c1dfe0c,
            ],
        ),
        (
            0xc1b5c37dfd2e8a4f,
            &[
                0xd7f31246, 0x551d4be5, 0x8597cae2, 0x2a5c5da6, 0xd2558e2c, 0x88ca31bf, 0x56e4c32e,
                0x2d3f60bd, 0xe581d369, 0xa389f565, 0xa0863a92, 0x6cc37165, 0xcf9968c9, 0x24a3bf40,
                0xa23fdc1f,
            ],
        ),
        (
            0xc5b7534314657ec2,
            &[
                0x13c75f17, 0x8a075434, 0x31c80a53, 0x16598da4, 0xf13dce30, 0xfc5f56be, 0xa039de00,
                0xd8959877, 0xf864e7bb, 0x7f462273, 0x3c08e781, 0xd30fc88f, 0x4217ed98, 0xbeda749,
                0xdcb655d,
            ],
        ),
        (
            0xf129fd17547d4fe1,
            &[
                0xc86f9368, 0x5d9b1f0b, 0xb9d8ce92, 0x5f3ced6c, 0xe5428fb7, 0x4659ab9c, 0x42ad1555,
                0xcaea0eeb, 0x623e20b5, 0xb938f770, 0x89857111, 0x80362026, 0xe40ef78a, 0xf77d78e9,
                0x75ed85df,
            ],
        ),
        (
            0x7b061ce6a7d9f7ed,
            &[
                0xc5596b51, 0x307deda1, 0xee47d739, 0x130fb3c9, 0xfc4a46f2, 0xbe1871f0, 0xfb26c5a4,
                0x64dee1d0, 0x9689b8f5, 0x89b4e450, 0x1de6f31a, 0x7844af43, 0x88eaf0c2, 0xa2fe02d6,
                0x8ca46360,
            ],
        ),
        (
            0x9c26eea55cfcef8e,
            &[
                0xcffcc0fa, 0x3f81de44, 0x6c6d8b15, 0xcee04284, 0x7076a9b4, 0xebb0002f, 0x5c76e45f,
                0x3980699f, 0xf09ce3b, 0xd9516e35, 0x6bbc1c13, 0x123548a5, 0x82db58f5, 0x753bb6f4,
                0x29910e47,
            ],
        ),
    ];

    #[test]
    fn test_f64() {
        for (seed, values) in F64_TESTS {
            let mut rand = LegacyRandomSource::new(*seed);

            for value in values.iter() {
                assert_eq!(*value, rand.next_f64());
            }
        }
    }

    #[test]
    fn test_u64() {
        for (seed, values) in U64_TESTS {
            let mut rand = LegacyRandomSource::new(*seed);

            for value in values.iter() {
                let rand = rand.next_u64();
                assert_eq!(*value, rand, "{:x} != {:x}", *value, rand);
            }
        }
    }

    #[test]
    fn test_u32() {
        for (seed, values) in U32_TESTS {
            let mut rand = LegacyRandomSource::new(*seed);

            for value in values.iter() {
                let rand = rand.next_u32();
                assert_eq!(*value, rand, "{:x} != {:x}", *value, rand);
            }
        }
    }
}
