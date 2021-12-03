#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    deref_nullptr
)]
#![allow(
    clippy::missing_safety_doc,
    clippy::unreadable_literal,
    clippy::cognitive_complexity,
    clippy::redundant_static_lifetimes,
    clippy::approx_constant,
    clippy::too_many_arguments,
    clippy::useless_transmute
)]

use classicube_sys::{
    cc_bool, ChatInputWidget, FontDesc, GfxResourceID, ScreenVTABLE, TextWidget, Widget,
};
use std::os::raw::{c_float, c_int};

#[repr(C)]
pub struct ChatScreen {
    pub VTABLE: *const ScreenVTABLE,
    pub grabsInput: cc_bool,
    pub blocksWorld: cc_bool,
    pub closable: cc_bool,
    pub dirty: cc_bool,
    pub maxVertices: ::std::os::raw::c_int,
    pub vb: GfxResourceID,
    pub widgets: *mut *mut Widget,
    pub numWidgets: ::std::os::raw::c_int,

    pub chatAcc: c_float,
    pub suppressNextPress: cc_bool,
    pub chatIndex: c_int,
    pub paddingX: c_int,
    pub paddingY: c_int,
    pub lastDownloadStatus: c_int,
    pub chatFont: FontDesc,
    pub announcementFont: FontDesc,
    pub bigAnnouncementFont: FontDesc,
    pub smallAnnouncementFont: FontDesc,
    pub announcement: TextWidget,
    pub bigAnnouncement: TextWidget,
    pub smallAnnouncement: TextWidget,
    pub input: ChatInputWidget,
    /* status: TextGroupWidget,
     * bottomRight: TextGroupWidget,
     * chat: TextGroupWidget,
     * clientStatus: TextGroupWidget,
     * altText: SpecialInputWidget,
     * Texture statusTextures[CHAT_MAX_STATUS];
     * Texture bottomRightTextures[CHAT_MAX_BOTTOMRIGHT];
     * Texture clientStatusTextures[CHAT_MAX_CLIENTSTATUS];
     * Texture chatTextures[TEXTGROUPWIDGET_MAX_LINES]; */
}
