use crystal_server_core::world;
use crystal_shared_proto::login::{
    CAddMember,
    CDelMember,
    CGroupInvite,
    CSwitchGroup,
};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_switch_group(
        &mut self,
        msg: CSwitchGroup,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::SetAllowGroup {
                session_id: self.session_id,
                allow: msg.allow_group,
            })
        };
        let _ = self.handle_world_events(events, out);

        let text = if msg.allow_group {
            "Group invitations enabled."
        } else {
            "Group invitations disabled."
        };

        self.send_system_chat(text, out);
    }

    pub(crate) fn handle_add_member(
        &mut self,
        msg: CAddMember,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = msg.name.trim();
        if name.is_empty() {
            self.send_system_chat(
                "Group system is not yet implemented on this Rust server.",
                out,
            );
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::InviteToParty {
                session_id: self.session_id,
                target_name: name.to_string(),
            })
        };
        let _ = self.handle_world_events(events, out);

        tracing::debug!(
            "AddMember request from session_id={} for name={} (group system not yet implemented)",
            self.session_id,
            msg.name,
        );

        self.send_system_chat(
            "Group system is not yet implemented on this Rust server.",
            out,
        );
    }

    pub(crate) fn handle_del_member(
        &mut self,
        msg: CDelMember,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = msg.name.trim();
        if name.is_empty() {
            self.send_system_chat(
                "Group system is not yet implemented on this Rust server.",
                out,
            );
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::KickFromParty {
                session_id: self.session_id,
                target_name: name.to_string(),
            })
        };
        let _ = self.handle_world_events(events, out);

        tracing::debug!(
            "DelMember request from session_id={} for name={} (group system not yet implemented)",
            self.session_id,
            msg.name,
        );

        self.send_system_chat(
            "Group system is not yet implemented on this Rust server.",
            out,
        );
    }

    pub(crate) fn handle_group_invite(
        &mut self,
        msg: CGroupInvite,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::RespondPartyInvite {
                session_id: self.session_id,
                accept: msg.accept_invite,
            })
        };
        let _ = self.handle_world_events(events, out);

        tracing::debug!(
            "GroupInvite response from session_id={} accept={} (group system not yet implemented)",
            self.session_id,
            msg.accept_invite,
        );

        self.send_system_chat(
            "Group system is not yet implemented on this Rust server.",
            out,
        );
    }
}

