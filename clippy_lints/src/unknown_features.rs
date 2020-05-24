use rustc_ast::ast::{Attribute, Crate, MacCall, MetaItem, MetaItemKind};
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_parse::{self, MACRO_ARGUMENTS};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::source_map::DUMMY_SP;

use crate::utils::{span_lint, span_lint_and_then};
use cargo_metadata::MetadataCommand;
use strsim::normalized_damerau_levenshtein;

declare_clippy_lint! {
    /// **What it does:** Finds references to features not defined in the cargo manifest file.
    ///
    /// **Why is this bad?** The referred feature will not be recognised and the related item will not be included
    /// by the conditional compilation engine.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// #[cfg(feature = "unknown")]
    /// fn example() { }
    /// ```
    pub UNKNOWN_FEATURES,
    cargo,
    "usage of features not defined in the cargo manifest file"
}

#[derive(Default)]
pub struct UnknownFeatures {
    features: FxHashSet<String>,
}

impl_lint_pass!(UnknownFeatures => [UNKNOWN_FEATURES]);

impl EarlyLintPass for UnknownFeatures {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _: &Crate) {
        fn transform_feature(name: &str, pkg: &str, local_pkg: &str) -> String {
            if pkg == local_pkg {
                name.into()
            } else {
                format!("{}/{}", pkg, name)
            }
        }

        let metadata = if let Ok(metadata) = MetadataCommand::new().exec() {
            metadata
        } else {
            span_lint(cx, UNKNOWN_FEATURES, DUMMY_SP, "could not read cargo metadata");
            return;
        };

        if let Some(local_pkg) = &cx.sess.opts.crate_name {
            for pkg in metadata.packages {
                self.features.extend(
                    pkg.features
                        .keys()
                        .map(|name| transform_feature(name, &pkg.name, local_pkg)),
                );
            }
        }
    }

    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        if attr.check_name(sym!(cfg)) {
            if let Some(item) = &attr.meta() {
                self.walk_cfg_metas(cx, item);
            }
        }
    }

    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &MacCall) {
        if mac.path == sym!(cfg) {
            let tts = mac.args.inner_tokens();
            let mut parser = rustc_parse::stream_to_parser(&cx.sess.parse_sess, tts, MACRO_ARGUMENTS);
            if let Ok(item) = parser.parse_meta_item() {
                self.walk_cfg_metas(cx, &item);
            }
        }
    }
}

impl UnknownFeatures {
    fn walk_cfg_metas(&mut self, cx: &EarlyContext<'_>, item: &MetaItem) {
        match &item.kind {
            MetaItemKind::List(items) => {
                for nested in items {
                    if let Some(item) = nested.meta_item() {
                        self.walk_cfg_metas(cx, item);
                    }
                }
            },
            MetaItemKind::NameValue(lit) if item.name_or_empty().as_str() == "feature" => {
                if let Some(value) = item.value_str() {
                    let feature = &*value.as_str();
                    if !self.features.contains(feature) {
                        let message = format!("unknown feature `{}`", feature);
                        span_lint_and_then(cx, UNKNOWN_FEATURES, lit.span, &message, |diag| {
                            if let Some(similar_name) = self.find_similar_name(feature) {
                                diag.span_suggestion(
                                    lit.span,
                                    "a feature with a similar name exists",
                                    format!("\"{}\"", similar_name),
                                    Applicability::MaybeIncorrect,
                                );
                            }
                        });
                    }
                }
            },
            _ => {},
        }
    }

    fn find_similar_name(&self, name: &str) -> Option<String> {
        let mut similar: Vec<_> = self
            .features
            .iter()
            .map(|f| (f, normalized_damerau_levenshtein(name, f)))
            .filter(|(_, sim)| *sim >= 0.7)
            .collect();

        similar.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        similar.into_iter().next().map(|(f, _)| f.clone())
    }
}
