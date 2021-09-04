use serde::{Deserialize, Deserializer};
use std::env;
use std::hash::{Hash, Hasher};
use std::lazy::SyncLazy;
use std::path::PathBuf;

pub static CARGO_TARGET_DIR: SyncLazy<PathBuf> = SyncLazy::new(|| match env::var_os("CARGO_TARGET_DIR") {
    Some(v) => v.into(),
    None => env::current_dir().unwrap().join("target"),
});

pub static TARGET_LIB: SyncLazy<PathBuf> = SyncLazy::new(|| {
    if let Some(path) = option_env!("TARGET_LIBS") {
        path.into()
    } else {
        let mut dir = CARGO_TARGET_DIR.clone();
        if let Some(target) = env::var_os("CARGO_BUILD_TARGET") {
            dir.push(target);
        }
        dir.push(env!("PROFILE"));
        dir
    }
});

#[must_use]
pub fn is_rustc_test_suite() -> bool {
    option_env!("RUSTC_TEST_SUITE").is_some()
}

// from cargo/core/compiler/fingerprint.rs
#[derive(Debug, Deserialize)]
pub struct Fingerprint {
    pub rustc: u64,
    pub features: String,
    pub target: u64,
    pub profile: u64,
    pub path: u64,
    pub deps: Vec<DepFingerprint>,
    pub local: Vec<LocalFingerprint>,
    pub rustflags: Vec<String>,
    pub metadata: u64,
    pub config: u64,
    pub compile_kind: u64,
}
impl Fingerprint {
    pub fn get_hash(&self) -> u64 {
        #[allow(deprecated)]
        let mut hasher = core::hash::SipHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}
impl Hash for Fingerprint {
    fn hash<H: Hasher>(&self, h: &mut H) {
        (
            self.rustc,
            &self.features,
            self.target,
            self.path,
            self.profile,
            &self.local,
            self.metadata,
            self.config,
            self.compile_kind,
            &self.rustflags,
        )
            .hash(h);

        h.write_usize(self.deps.len());
        for dep in &self.deps {
            dep.pkg_id.hash(h);
            dep.name.hash(h);
            dep.public.hash(h);
            h.write_u64(dep.fingerprint);
        }
    }
}

#[derive(Debug)]
pub struct DepFingerprint {
    pub pkg_id: u64,
    pub name: String,
    pub public: bool,
    pub fingerprint: u64,
}
impl<'d> Deserialize<'d> for DepFingerprint {
    fn deserialize<D: Deserializer<'d>>(d: D) -> Result<Self, D::Error> {
        let (pkg_id, name, public, fingerprint) = <(u64, String, bool, u64)>::deserialize(d)?;
        Ok(Self {
            pkg_id,
            name,
            public,
            fingerprint,
        })
    }
}

#[derive(Debug, Deserialize, Hash)]
pub enum LocalFingerprint {
    Precalculated(String),
    CheckDepInfo { dep_info: PathBuf },
    RerunIfChanged { output: PathBuf, paths: Vec<PathBuf> },
    RerunIfEnvChanged { var: String, val: Option<String> },
}
