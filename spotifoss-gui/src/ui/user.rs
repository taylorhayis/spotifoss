use druid::{
    Data, LensExt, Selector, Widget, WidgetExt, commands,
    widget::{Flex, Label, ViewSwitcher},
};

use crate::{
    data::{AppState, Library, UserProfile},
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, icons, icons::SvgIcon},
};

use super::theme;

pub const LOAD_PROFILE: Selector = Selector::new("app.user.load-profile");

pub fn user_widget() -> impl Widget<AppState> {
    // Shannon/librespot streaming link — opened lazily when playback needs it, not at login.
    let session_status = ViewSwitcher::new(
        |state: &AppState, _| {
            if !state.config.has_credentials() {
                0
            } else if state.session.is_connected() {
                2
            } else {
                1
            }
        },
        |mode, _, _| match mode {
            0 => Label::new("Not signed in")
                .with_text_color(theme::STATUS_TEXT_COLOR)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .boxed(),
            1 => Empty.boxed(),
            2 => Label::new("Streaming")
                .with_text_color(theme::STATUS_TEXT_COLOR)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .boxed(),
            _ => Empty.boxed(),
        },
    );

    let user_profile = Async::new(
        || Empty,
        || {
            Label::raw()
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(UserProfile::display_name)
        },
        || Empty,
    )
    .lens(AppState::library.then(Library::user_profile.in_arc()))
    .on_command_async(
        LOAD_PROFILE,
        |_| WebApi::global().get_user_profile(),
        |_, data, d| data.with_library_mut(|l| l.user_profile.defer(d)),
        |_, data, r| data.with_library_mut(|l| l.user_profile.update(r)),
    );

    Flex::row()
        .with_child(
            Flex::column()
                .with_child(session_status)
                .with_default_spacer()
                .with_child(user_profile)
                .padding(theme::grid(1.0)),
        )
        .with_child(preferences_widget(&icons::PREFERENCES))
}

fn preferences_widget<T: Data>(svg: &SvgIcon) -> impl Widget<T> {
    svg.scale((theme::grid(3.0), theme::grid(3.0)))
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, _, _| ctx.submit_command(commands::SHOW_PREFERENCES))
}
