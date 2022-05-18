use crate::prelude::*;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lod(pub ParameterIO);

impl Convertible<ParameterIO> for Lod {}

impl From<&ParameterIO> for Lod {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for Lod {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<Lod> for ParameterIO {
    fn from(val: Lod) -> Self {
        val.0
    }
}

impl SimpleMergeableAamp for Lod {
    fn inner(&self) -> &ParameterIO {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let lod = super::Lod::try_from(&pio).unwrap();
        let data = lod.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let lod2 = super::Lod::try_from(&pio2).unwrap();
        assert_eq!(lod, lod2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let lod = super::Lod::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let lod2 = super::Lod::try_from(&pio2).unwrap();
        let diff = lod.diff(&lod2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let lod = super::Lod::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let lod2 = super::Lod::try_from(&pio2).unwrap();
        let diff = lod.diff(&lod2);
        let merged = lod.merge(&diff);
        assert_eq!(lod2, merged);
    }
}