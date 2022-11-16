use super::ResolvedDisallowedFunction;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{def_id::DefId, definitions::DefPathHash};
use rustc_macros::{Decodable, Encodable};
use rustc_middle::ty::TyCtxt;
use rustc_serialize::{
    opaque::{FileEncoder, MemDecoder},
    Decodable, Encodable,
};
use rustc_span::def_id::StableCrateId;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const CRATE_INFO_DIR: &str = "disallowed_from_async";

#[derive(Debug, Decodable, Encodable)]
pub struct TaintedFunction {
    /// Call stack leading to another disallowed call (bottom-up order).
    callstack: Vec<DefPathHash>,
}

#[derive(Debug, Decodable, Encodable)]
pub struct CrateInfo {
    stable_crate_id: StableCrateId,
    tainted_functions: FxHashMap<DefPathHash, TaintedFunction>,
}

impl CrateInfo {
    pub fn new(stable_crate_id: StableCrateId) -> Self {
        Self {
            stable_crate_id,
            tainted_functions: FxHashMap::default(),
        }
    }

    pub fn record_tainted_function(&mut self, tcx: TyCtxt<'_>, def_path_hash: DefPathHash, callstack: &[DefId]) {
        self.tainted_functions.insert(
            def_path_hash,
            TaintedFunction {
                callstack: callstack.iter().map(|def_id| tcx.def_path_hash(*def_id)).collect(),
            },
        );
    }

    // FIXME(sproul): solve nasty lifetime bounds and return impl Iterator
    pub fn get_tainted_function_def_ids(&self, tcx: TyCtxt<'_>) -> Vec<ResolvedDisallowedFunction> {
        // FIXME(sproul): this error handling is quite nasty
        let mut err_handler = || panic!("def ID look-up failed");
        self.tainted_functions
            .iter()
            .map(move |(def_path_hash, tainted_fn)| {
                let fn_def_id = tcx.def_path_hash_to_def_id(*def_path_hash, &mut err_handler);
                let callstack = tainted_fn
                    .callstack
                    .iter()
                    .map(|def_path_hash| tcx.def_path_hash_to_def_id(*def_path_hash, &mut err_handler))
                    .collect();
                ResolvedDisallowedFunction { fn_def_id, callstack }
            })
            .collect()
    }

    pub fn crate_path(base_dir: &Path) -> PathBuf {
        base_dir.join(CRATE_INFO_DIR)
    }

    pub fn store(&self, base_dir: &Path) -> Result<(), String> {
        let crate_path = Self::crate_path(base_dir);
        create_dir_all(&crate_path).map_err(|e| format!("unable to create {}: {}", crate_path.display(), e))?;

        let path = crate_path.join(format!("{}.bin", self.stable_crate_id.to_u64()));

        let mut encoder =
            FileEncoder::new(&path).map_err(|e| format!("error opening {} for writing: {}", path.display(), e))?;
        self.encode(&mut encoder);
        encoder.flush();
        Ok(())
    }

    pub fn load(base_dir: &Path, stable_crate_id: StableCrateId) -> Result<Self, String> {
        let path = Self::crate_path(base_dir).join(format!("{}.bin", stable_crate_id.to_u64()));
        let mut bytes = vec![];
        File::open(&path)
            .and_then(|mut f| f.read_to_end(&mut bytes))
            .map_err(|e| format!("error reading crate info from {}: {}", path.display(), e))?;
        let mut decoder = MemDecoder::new(&bytes, 0);
        Ok(Self::decode(&mut decoder))
    }
}
