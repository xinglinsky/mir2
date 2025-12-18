use bevy::prelude::*;
use crystal_client_net::NetClient;
use crystal_shared_proto::select::SelectInfo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoginField {
    Account,
    Password,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Resource)]
pub(crate) enum LoginUiStage {
    Login,
    Select,
    InGame,
}

#[derive(Resource)]
pub(crate) struct LoginUiNetState {
    pub(crate) server_addr: String,
    pub(crate) net: Option<NetClient>,
    pub(crate) connected: bool,
    pub(crate) just_connected: bool,
    pub(crate) sent_version: bool,
    pub(crate) version_checked: bool,
    pub(crate) pending_login: Option<(String, String)>,
    pub(crate) logged_in: bool,
    pub(crate) characters: Vec<SelectInfo>,
    pub(crate) selected_character_index: Option<i32>,
    pub(crate) pending_start_game: Option<i32>,
    pub(crate) start_game_ok: bool,
    pub(crate) map_index: Option<i32>,
    pub(crate) map_file_name: Option<String>,
    pub(crate) user_name: Option<String>,
    pub(crate) user_location_x: Option<i32>,
    pub(crate) user_location_y: Option<i32>,
    pub(crate) last_error: Option<String>,
}

#[derive(Event)]
pub(crate) struct LoginUiStartLogin;

#[derive(Resource)]
pub(crate) struct LoginUiState {
    pub(crate) account: String,
    pub(crate) password: String,
    pub(crate) focus: LoginField,
    pub(crate) bg_frame: usize,
    pub(crate) bg_timer: Timer,
}

#[derive(Resource)]
pub(crate) struct LoginUiAssets {
    pub(crate) bg_frames: Vec<Handle<Image>>,
    pub(crate) dialog_bg: Handle<Image>,
    pub(crate) dialog_w: u32,
    pub(crate) dialog_h: u32,
    pub(crate) title_login: Handle<Image>,
    pub(crate) title_login_w: u32,
    pub(crate) label_id: Handle<Image>,
    pub(crate) label_pass: Handle<Image>,
    pub(crate) btn_ok: ButtonTriplet,
    pub(crate) btn_new: ButtonTriplet,
    pub(crate) btn_change_pass: ButtonTriplet,
    pub(crate) btn_safe: ButtonTriplet,
    pub(crate) btn_cancel: ButtonTriplet,

    pub(crate) select_bg: Handle<Image>,
    pub(crate) select_title: Handle<Image>,
    pub(crate) select_title_size: (u32, u32),
    pub(crate) select_btn_start: ButtonTriplet,
    pub(crate) select_btn_start_size: (u32, u32),
    pub(crate) select_btn_new: ButtonTriplet,
    pub(crate) select_btn_new_size: (u32, u32),
    pub(crate) select_btn_delete: ButtonTriplet,
    pub(crate) select_btn_delete_size: (u32, u32),
    pub(crate) select_btn_credits: ButtonTriplet,
    pub(crate) select_btn_credits_size: (u32, u32),
    pub(crate) select_btn_exit: ButtonTriplet,
    pub(crate) select_btn_exit_size: (u32, u32),
    pub(crate) select_slot_empty: Handle<Image>,
    pub(crate) select_slot_size: (u32, u32),
    pub(crate) select_slot_unselected: Vec<Handle<Image>>,
    pub(crate) select_slot_selected: Vec<Handle<Image>>,
    pub(crate) select_char_frames: Vec<Vec<Handle<Image>>>,
    pub(crate) select_char_offsets: Vec<Vec<(i16, i16)>>,
    pub(crate) select_char_overlay_frames: Vec<Vec<Handle<Image>>>,
    pub(crate) select_char_overlay_offsets: Vec<Vec<(i16, i16)>>,
}

#[derive(Clone)]
pub(crate) struct ButtonTriplet {
    pub(crate) base: Handle<Image>,
    pub(crate) hover: Handle<Image>,
    pub(crate) pressed: Handle<Image>,
}

#[derive(Component)]
pub(crate) struct LoginBackground;

#[derive(Component)]
pub(crate) struct LoginUiRoot;

#[derive(Component)]
pub(crate) struct LoginUiLoginRoot;

#[derive(Component)]
pub(crate) struct LoginUiSelectRoot;

#[derive(Component)]
pub(crate) struct SelectStartButton;

#[derive(Component)]
pub(crate) struct SelectExitButton;

#[derive(Component)]
pub(crate) struct SelectNewCharacterButton;

#[derive(Component)]
pub(crate) struct SelectDeleteCharacterButton;

#[derive(Component)]
pub(crate) struct SelectCreditsButton;

#[derive(Component)]
pub(crate) struct SelectLastAccessText;

#[derive(Component)]
pub(crate) struct SelectStatusText;

#[derive(Component)]
pub(crate) struct SelectSlotImage;

#[derive(Component)]
pub(crate) struct SelectSlotNameText;

#[derive(Component)]
pub(crate) struct SelectSlotLevelText;

#[derive(Component)]
pub(crate) struct SelectSlotClassText;

#[derive(Component)]
pub(crate) struct SelectCharacterDisplay;

#[derive(Component)]
pub(crate) struct SelectCharacterBaseImage;

#[derive(Component)]
pub(crate) struct SelectCharacterOverlayImage;

#[derive(Component)]
pub(crate) struct SelectCharacterDisplayAnim {
    pub(crate) timer: Timer,
    pub(crate) frame: usize,
    pub(crate) key: Option<(u8, u8)>,
}

#[derive(Component)]
pub(crate) struct SelectSlotButton(pub(crate) usize);

#[derive(Component)]
pub(crate) struct InGameStatusText;

#[derive(Component)]
pub(crate) struct LoginText(pub(crate) LoginField);

#[derive(Component)]
pub(crate) struct LoginInputArea(pub(crate) LoginField);

#[derive(Component)]
pub(crate) struct LoginInputFrame(pub(crate) LoginField);

#[derive(Component)]
pub(crate) struct LoginCaret;

#[derive(Component)]
pub(crate) struct LoginOkButton;

#[derive(Component)]
pub(crate) struct ButtonEnabled(pub(crate) bool);

#[derive(Resource)]
pub(crate) struct CaretBlink {
    pub(crate) timer: Timer,
    pub(crate) visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoginButton {
    Ok,
    Account,
    Pass,
    ViewKey,
    Close,
}

#[derive(Component)]
pub(crate) struct LoginButtonKind(pub(crate) LoginButton);

#[derive(Component)]
pub(crate) struct ButtonSkin {
    pub(crate) base: Handle<Image>,
    pub(crate) hover: Handle<Image>,
    pub(crate) pressed: Handle<Image>,
}

#[derive(Component)]
pub(crate) struct ButtonImage;

pub(crate) fn is_valid_account(s: &str) -> bool {
    // Globals.cs: MinAccountIDLength=3, MaxAccountIDLength=15
    let len = s.chars().count();
    if len < 3 || len > 15 {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric())
}

pub(crate) fn is_valid_password(s: &str) -> bool {
    // Globals.cs: MinPasswordLength=5, MaxPasswordLength=15
    let len = s.chars().count();
    if len < 5 || len > 15 {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric())
}
