use gen_core::ChunkGenerator;
use std::sync::Arc;
use tracing::warn;

pub fn generator_from_name(name: &str, seed: u64) -> Option<Arc<dyn ChunkGenerator>> {
    match (name.trim().to_ascii_lowercase().as_str(), seed) {
        // Easter egg: this seed forces skyblock regardless of configured name.
        // Note: rustrover seems to complain that these arms are unreachable, but they aren't
        (_, 0x43f6c73858579990) | ("skyblock", _) => {
            Some(Arc::new(skyblock::SkyblockGenerator::new(seed)))
        }
        ("normal", _) => Some(Arc::new(normal::NormalGenerator::new(seed))),
        ("superflat", _) => Some(Arc::new(superflat::SuperflatGenerator::new(seed))),
        ("vanilla", _) => {
            warn!("The vanilla terrain generator is experimental! Use at your own risk!");
            Some(Arc::new(vanilla::VanillaGenerator::new(seed)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_superflat_generator() {
        let generator = generator_from_name("superflat", 0);

        assert!(generator.is_some());
        assert_eq!(generator.unwrap().id().as_str(), "superflat");
    }

    #[test]
    fn selects_skyblock_generator() {
        let generator = generator_from_name("", 4897320689626225040u64);

        assert!(generator.is_some());
        assert_eq!(generator.unwrap().id().as_str(), "skyblock");
    }

    #[test]
    fn selects_normal_generator() {
        let generator = generator_from_name("normal", 0);

        assert!(generator.is_some());
        assert_eq!(generator.unwrap().id().as_str(), "normal");
    }

    #[test]
    fn selects_skyblock_generator_works_from_name() {
        let generator = generator_from_name("skyblock", 0);

        assert!(generator.is_some());
        assert_eq!(generator.unwrap().id().as_str(), "skyblock");
    }
}
