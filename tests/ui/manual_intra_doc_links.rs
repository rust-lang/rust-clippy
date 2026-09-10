//@aux-build:external_item.rs
//@aux-build:external_consts.rs

//! Link with [refdef] and <https://docs.rs/external_item/latest/external_item/index.html>
//! and [inner link][refdef].
//!
//! [refdef]: index.html
//~^^^ manual_intra_doc_links
//~^^^^^ manual_intra_doc_links
//~| manual_intra_doc_links

#![warn(clippy::manual_intra_doc_links)]

pub mod extern_crate {
    pub(crate) extern crate external_item;
}

/// Link to [external item](https://docs.rs/external_item/latest/external_item/struct._ExternalStruct.html)
//~^ manual_intra_doc_links
pub struct DocsrsLinkExtern;

/// Link to [external item](https://docs.rs/external_item/latest/external_item/index.html)
//~^ manual_intra_doc_links
pub struct DocsrsLinkExternIndexHtmlCrate;

/// Link to [external item](https://docs.rs/external_item/latest/external_item/module/index.html)
//~^ manual_intra_doc_links
pub struct DocsrsLinkExternIndexHtmlModule;

/// Link to [a crate that isn't a dependency](https://docs.rs/regex/latest/regex/index.html)
pub struct DocsrsLinkNonDependency;

use external_consts::MAGIC_NUMBER;
/// Link to [external const](https://docs.rs/external_consts/latest/external_consts/constant.MAGIC_NUMBER.html)
//~^ manual_intra_doc_links
pub struct DocsrsLinkUse;
/// Link to [external const](https://docs.rs/external_consts/latest/external_consts/index.html)
//~^ manual_intra_doc_links
pub struct DocsrsLinkUseIndexHtmlCrate;
/// Link to [external const](https://docs.rs/external_consts/latest/external_consts/module/index.html)
//~^ manual_intra_doc_links
pub struct DocsrsLinkUseIndexHtmlModule;

/// Link to [external item](../external_item/struct._ExternalStruct.html)
//~^ manual_intra_doc_links
pub struct RelativeLinkExtern;
/// Link to [external item](../external_item/index.html)
//~^ manual_intra_doc_links
pub struct RelativeLinkExternIndexHtmlCrate;
/// Link to [external item](../external_item/module/index.html)
//~^ manual_intra_doc_links
pub struct RelativeLinkExternIndexHtmlModule;

/// Link to [external item](../external_consts/constant.MAGIC_NUMBER.html)
//~^ manual_intra_doc_links
pub struct RelativeLinkUse;
/// Link to [external item](../external_consts/index.html)
//~^ manual_intra_doc_links
pub struct RelativeLinkUseIndexHtmlCrate;
/// Link to [external item](../external_consts/module/index.html)
//~^ manual_intra_doc_links
pub struct RelativeLinkUseIndexHtmlModule;

/// Link to [local item](struct.FooBar.html)
//~^ manual_intra_doc_links
pub struct RelativeLinkLocalItem;

/// Link to [myself](index.html)
//~^ manual_intra_doc_links
pub mod my_mod {
    /// Link to [local item](struct.FooBar.html)
    //~^ manual_intra_doc_links
    pub struct RelativeLinkLocalSubmoduleItem;
    /// Link to [local item](../struct.FooBar.html)
    //~^ manual_intra_doc_links
    pub struct RelativeLinkLocalParentItem;
    /// Link to [local item](mod_2/struct.FooBar.html)
    //~^ manual_intra_doc_links
    pub struct RelativeLinkLocalChildModItem;
    /// Link to [local item](index.html)
    //~^ manual_intra_doc_links
    pub struct RelativeLinkLocalModule;
    /// Link to [local item](../index.html)
    //~^ manual_intra_doc_links
    pub struct RelativeLinkLocalParentCrate;
    /// Link to [outside the docs](../../index.html)
    pub struct RelativeLinkLocalRoot;

    pub struct FooBar;
    pub mod mod_2 {
        pub struct FooBar;
    }

    /// Link to [external item](https://docs.rs/external_item/latest/external_item/struct._ExternalStruct.html)
    //~^ manual_intra_doc_links
    pub struct DocsrsLinkExtern;
    /// Link to [external const](https://docs.rs/external_consts/latest/external_consts/constant.MAGIC_NUMBER.html)
    //~^ manual_intra_doc_links
    pub struct DocsrsLinkUse;
    /// Link to [external item](../../external_item/struct._ExternalStruct.html)
    //~^ manual_intra_doc_links
    pub struct RelativeLinkExtern;
    /// Link to [external item](../../external_consts/constant.MAGIC_NUMBER.html)
    //~^ manual_intra_doc_links
    pub struct RelativeLinkUse;
}

pub struct FooBar;

/// Link to [a crate that isn't a dependency](../nonexistant/index.html)
pub struct RelativeLinkNonDependency;

/// Link to [external item](../constant.MAGIC_NUMBER.html)
pub struct RelativeLinkOutsideCrate;
/// Link to [external item](../index.html)
pub struct RelativeLinkOutsideCrateIndexHtml;
