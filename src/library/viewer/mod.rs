// This file is part of Memories.
//
// Copyright (c) 2024 Max Rodriguez
// All rights reserved.
//
// Memories is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Memories is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Memories.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod imp;

use crate::window::MrsApplicationWindow;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::{clone, g_debug, g_error};
#[cfg(feature = "disable-glycin-sandbox")]
use glycin::SandboxMechanism;
use gtk::{gdk, gio, glib};
use std::ffi::OsStr;

/// Enum that represents the types of content that
/// can be displayed by the `MrsViewer` object.
#[derive(Debug, Clone)]
pub enum ViewerContentType {
    VectorGraphics,
    Image,
    Video,
    Invalid,
}

impl ViewerContentType {
    /// Returns a `ViewerContentType` enum that matches the file extension given.
    pub fn from_ext(extension: &OsStr) -> ViewerContentType {
        let ext_str: &str = &extension.to_str().unwrap().to_lowercase();

        match ext_str {
            "svg" => ViewerContentType::VectorGraphics,
            "png" | "jpg" | "jpeg" | "webp" | "heic" | "heif" => ViewerContentType::Image,
            "mp4" | "webm" | "mkv" | "mov" | "avi" | "gif" => ViewerContentType::Video,
            _ => {
                g_debug!("ViewerContentType", "from_ext() received invalid file extension.");
                ViewerContentType::Invalid
            }
        }
    }
}

glib::wrapper! {
    pub struct MrsViewer(ObjectSubclass<imp::MrsViewer>)
        @extends gtk::Widget, adw::Bin;
}

#[gtk::template_callbacks]
impl MrsViewer {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn window(&self) -> MrsApplicationWindow {
        self.root()
            .expect("Must be in a GtkApplicationWindow.")
            .downcast()
            .expect("Failed to downcast to MrsApplicationWindow.")
    }

    /// This function is public so that it can be called once we
    /// are placed in the widget tree and can access the window.
    pub fn setup_gactions(&self) {
        let win: MrsApplicationWindow = self.window();
        let actions = gio::SimpleActionGroup::new();

        let action_close = gio::ActionEntry::builder("show-properties")
            .activate(
                clone!(@weak self as viewer => move |_: &gio::SimpleActionGroup, _, _| {
                    viewer.imp()
                        .split_view
                        .set_show_sidebar(!viewer.imp().split_view.shows_sidebar());
                }),
            )
            .build();

        actions.add_action_entries([action_close]);
        win.insert_action_group("viewer", Some(&actions));
    }

    /// Sets the content type setting for the viewer page.
    /// The `ViewerContentType` enum given directly correlates
    /// to a stack page that has the proper widget for the content.
    pub fn set_content_type(&self, content_type: &ViewerContentType) {
        match content_type {
            ViewerContentType::VectorGraphics => self.imp().viewer_stack.set_visible_child_name("image"),
            ViewerContentType::Image => self.imp().viewer_stack.set_visible_child_name("image"),
            ViewerContentType::Video => self.imp().viewer_stack.set_visible_child_name("video"),
            _ => g_debug!("Viewer", "Received invalid ViewerContentType enum!"),
        }
    }

    pub fn set_content_file(&self, file: &gio::File) {
        match self.imp().viewer_stack.visible_child_name().unwrap().as_str() {
            "render" => self.imp().viewer_picture.set_file(Some(file)),
            "image" => {
                glib::spawn_future_local(clone!(@weak self as viewer, @strong file => async move {
                    #[allow(unused_mut)]
                    let mut glycin_loader: glycin::Loader = glycin::Loader::new(file);

                    #[cfg(feature = "disable-glycin-sandbox")]
                    glycin_loader.sandbox_mechanism(Some(SandboxMechanism::NotSandboxed));

                    let image: glycin::Image = glycin_loader.load().await.expect("FIXME");
                    let texture: gdk::Texture = image.next_frame().await.expect("FIXME").texture;

                    viewer.imp().viewer_picture.set_paintable(Some(&texture));
                }));
            }
            "video" => self.imp().viewer_video.set_file(Some(file)),
            _ => g_error!("Viewer", "Found unexpected visible child name in viewer stack."),
        }
    }

    /// Returns a new `AdwNavigationPage` object that
    /// has its child set to the `&self` GObject.
    pub fn wrap_in_navigation_page(&self) -> adw::NavigationPage {
        let new_navigation_page: adw::NavigationPage = adw::NavigationPage::builder()
            .title(gettext("Loading Content"))
            .child(self)
            .build();
        new_navigation_page
    }

    #[template_callback]
    fn fullscreen_toggle(&self, button: &gtk::ToggleButton) {
        let fullscreen: bool = self.window().is_fullscreened();
        self.window().set_fullscreened(!fullscreen);

        if !fullscreen {
            button.set_tooltip_text(Some(&gettext("Exit Fullscreen")));
        } else {
            button.set_tooltip_text(Some(&gettext("View Fullscreen")));
        }
    }
}

impl Default for MrsViewer {
    fn default() -> Self {
        Self::new()
    }
}
