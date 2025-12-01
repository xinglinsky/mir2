use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_shared_proto::login::{
    CAddMember,
    CDelMember,
    CGroupInvite,
    CSwitchGroup,
};
use crystal_shared_proto::user::group::{
    SAddMember,
    SDeleteGroup,
    SDeleteMember,
    SGroupInvite,
    SGroupMembersMap,
    SSendMemberLocation,
    SSwitchGroup,
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

        // Snapshot current party membership before applying the toggle so we
        // can detect whether this change caused the player to leave their
        // party (AllowGroup off while grouped), mirroring C# SwitchGroup /
        // LeaveGroup semantics.
        let party_members_before = {
            let world = self.world.lock().unwrap();
            world.party_members_for_session(self.session_id)
        };

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::SetAllowGroup {
                session_id: self.session_id,
                allow: msg.allow_group,
            })
        };
        let _ = self.handle_world_events(events, out);

        // Notify the client so the group UI toggle stays in sync.
        let switch_pkt = SSwitchGroup {
            allow_group: msg.allow_group,
        };
        if let Ok(raw) = switch_pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        let text = if msg.allow_group {
            "已开启组队邀请。"
        } else {
            "已关闭组队邀请。"
        };

        self.send_system_chat(text, out);

        // If the player disabled group invites and was in a party, the world
        // side will have called leave_party; notify remaining members so their
        // group UI stays in sync.
        if !msg.allow_group {
            if let Some(members_before) = party_members_before {
                let members_after = {
                    let world = self.world.lock().unwrap();
                    world.party_members_for_session(self.session_id)
                };

                if members_after.is_none() {
                    let total = members_before.len();
                    if total >= 2 {
                        let leaver_sid = self.session_id;
                        let leaving_name = members_before
                            .iter()
                            .find(|(sid, _)| *sid == leaver_sid)
                            .map(|(_, name)| name.clone())
                            .unwrap_or_else(String::new);

                        let mut outboxes = self.outboxes.lock().unwrap();
                        if total > 2 && !leaving_name.is_empty() {
                            let pkt = SDeleteMember {
                                name: leaving_name.clone(),
                            };
                            if let Ok(raw) = pkt.encode() {
                                let encoded = Self::encode_raw(raw);
                                for (sid, _) in &members_before {
                                    if *sid == leaver_sid {
                                        continue;
                                    }
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(encoded.clone());
                                }
                            }
                        } else {
                            let pkt = SDeleteGroup;
                            let encoded = Self::encode_raw(pkt.encode());
                            for (sid, _) in &members_before {
                                if *sid == leaver_sid {
                                    continue;
                                }
                                outboxes
                                    .entry(*sid)
                                    .or_default()
                                    .push(encoded.clone());
                            }
                        }
                    }
                }
            }
        }
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
            self.send_system_chat("请输入玩家名字。", out);
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

        // If the world recorded a pending invite for the target from this
        // session, send an SGroupInvite packet to the invitee so the client
        // shows the invite dialog.
        let (inviter_name_opt, invitee_session_id_opt) = {
            let world = self.world.lock().unwrap();
            let inviter_name = world.player_name(self.session_id);
            let invitee_session_id = world.find_session_by_name(name).and_then(|sid| {
                match world.pending_group_invite_from(sid) {
                    Some(from) if from == self.session_id => Some(sid),
                    _ => None,
                }
            });
            (inviter_name, invitee_session_id)
        };

        if let (Some(inviter_name), Some(invitee_sid)) = (inviter_name_opt, invitee_session_id_opt)
        {
            let invite_pkt = SGroupInvite { name: inviter_name };
            if let Ok(raw) = invite_pkt.encode() {
                let encoded = Self::encode_raw(raw);
                if invitee_sid == self.session_id {
                    out.push(encoded);
                } else {
                    let mut outboxes = self.outboxes.lock().unwrap();
                    outboxes.entry(invitee_sid).or_default().push(encoded);
                }
            }
            tracing::debug!(
                "AddMember request from session_id={} for name={} -> invite sent to session_id={}",
                self.session_id,
                msg.name,
                invitee_sid,
            );
        } else {
            // World rejected or did not record the invite (target offline,
            // disallowing group, already grouped, cooldown, etc.). Detailed
            // error messages (if any) are emitted via WorldEvent::PartySystemMessage.
            tracing::debug!(
                "AddMember request from session_id={} for name={} failed (no pending invite recorded)",
                self.session_id,
                msg.name,
            );
        }
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
                "请输入玩家名字。",
                out,
            );
            return;
        }

        // Snapshot party membership before processing the kick so we can
        // compare after and notify all affected members if the kick succeeds.
        let (members_before, target_sid_opt) = {
            let world = self.world.lock().unwrap();
            let members = world.party_members_for_session(self.session_id);
            let target_sid = world.find_session_by_name(name);
            (members, target_sid)
        };

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::KickFromParty {
                session_id: self.session_id,
                target_name: name.to_string(),
            })
        };
        let _ = self.handle_world_events(events, out);

        // Re-check party membership from the leader's perspective to confirm
        // that the target was actually removed before emitting any packets.
        let kicked = {
            if let (Some(ref before), Some(target_sid)) = (&members_before, target_sid_opt) {
                let world = self.world.lock().unwrap();
                let after = world
                    .party_members_for_session(self.session_id)
                    .unwrap_or_default();
                let was_in_party = before.iter().any(|(sid, _)| *sid == target_sid);
                let still_in_party = after.iter().any(|(sid, _)| *sid == target_sid);
                was_in_party && !still_in_party
            } else {
                false
            }
        };

        if kicked {
            let (members_before, target_sid) = (members_before.unwrap(), target_sid_opt.unwrap());
            // Use the canonical name from world state if available.
            let removed_name = members_before
                .iter()
                .find(|(sid, _)| *sid == target_sid)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| name.to_string());

            // Notify the kicked player that their group has been removed.
            let encoded_delete_group = {
                let pkt = SDeleteGroup;
                Self::encode_raw(pkt.encode())
            };
            if target_sid == self.session_id {
                out.push(encoded_delete_group.clone());
            } else {
                let mut outboxes = self.outboxes.lock().unwrap();
                outboxes
                    .entry(target_sid)
                    .or_default()
                    .push(encoded_delete_group.clone());
            }

            // Notify remaining members to remove the kicked member from their
            // group list.
            let del_member_pkt = SDeleteMember {
                name: removed_name.clone(),
            };
            if let Ok(raw) = del_member_pkt.encode() {
                let encoded = Self::encode_raw(raw);
                let mut outboxes = self.outboxes.lock().unwrap();
                for (sid, _) in members_before {
                    if sid == target_sid {
                        continue;
                    }
                    if sid == self.session_id {
                        out.push(encoded.clone());
                    } else {
                        outboxes.entry(sid).or_default().push(encoded.clone());
                    }
                }
            }
            tracing::debug!(
                "DelMember request from session_id={} for name={} -> kicked",
                self.session_id,
                msg.name,
            );
        } else {
            tracing::debug!(
                "DelMember request from session_id={} for name={} failed (no membership change)",
                self.session_id,
                msg.name,
            );
        }
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

        // On accept, query the resulting party membership and send SAddMember
        // packets so all members see the updated group list.
        if msg.accept_invite {
            let members = {
                let world = self.world.lock().unwrap();
                world.party_members_for_session(self.session_id)
            };

            if let Some(members) = members {
                // First, mirror the legacy behaviour of sending SAddMember
                // for all members to each participant so their group list is
                // fully populated.
                let names: Vec<String> = members.iter().map(|(_, n)| n.clone()).collect();
                let mut outboxes = self.outboxes.lock().unwrap();

                for (sid, _) in &members {
                    for name in &names {
                        let add_pkt = SAddMember {
                            name: name.clone(),
                        };
                        if let Ok(raw) = add_pkt.encode() {
                            let encoded = Self::encode_raw(raw);
                            if *sid == self.session_id {
                                out.push(encoded.clone());
                            } else {
                                outboxes.entry(*sid).or_default().push(encoded.clone());
                            }
                        }
                    }
                }

                // Then, send GroupMembersMap and SendMemberLocation packets so
                // the client-side big map UI can show party member locations
                // and map names, mirroring C# GroupMemberMapNameChanged and
                // GetPlayerLocation.
                drop(outboxes);
                self.broadcast_group_maps_for_self(out);
                self.broadcast_group_locations_for_self(out);
            }
        }

        tracing::debug!(
            "GroupInvite response from session_id={} accept={}",
            self.session_id,
            msg.accept_invite,
        );
    }

    pub(crate) fn broadcast_group_maps_for_self(&mut self, out: &mut Vec<Vec<u8>>) {
        // Snapshot current party members and their map titles.
        let member_infos = {
            let mut infos = Vec::new();
            let world = self.world.lock().unwrap();
            let members = match world.party_members_for_session(self.session_id) {
                Some(m) if !m.is_empty() => m,
                _ => return,
            };

            for (sid, name) in members {
                if let Some((map_index, _, _, _)) = world.player_position(sid) {
                    if let Some(mi) = self.world_db.get_map_info(map_index) {
                        infos.push((sid, name.clone(), mi.title.clone()));
                    }
                }
            }
            infos
        };

        if member_infos.is_empty() {
            return;
        }

        let mut outboxes = self.outboxes.lock().unwrap();
        let sids: Vec<world::SessionId> = member_infos.iter().map(|(sid, _, _)| *sid).collect();

        for (_, name, map_title) in &member_infos {
            let pkt = SGroupMembersMap {
                player_name: name.clone(),
                player_map: map_title.clone(),
            };
            if let Ok(raw) = pkt.encode() {
                let encoded = Self::encode_raw(raw);
                for sid in &sids {
                    if *sid == self.session_id {
                        out.push(encoded.clone());
                    } else {
                        outboxes.entry(*sid).or_default().push(encoded.clone());
                    }
                }
            }
        }
    }

    pub(crate) fn broadcast_group_locations_for_self(&mut self, out: &mut Vec<Vec<u8>>) {
        // Snapshot current party members and their positions.
        let member_infos = {
            let mut infos = Vec::new();
            let world = self.world.lock().unwrap();
            let members = match world.party_members_for_session(self.session_id) {
                Some(m) if !m.is_empty() => m,
                _ => return,
            };

            for (sid, name) in members {
                if let Some((_, x, y, _)) = world.player_position(sid) {
                    infos.push((sid, name.clone(), x, y));
                }
            }
            infos
        };

        if member_infos.is_empty() {
            return;
        }

        let mut outboxes = self.outboxes.lock().unwrap();
        let sids: Vec<world::SessionId> = member_infos.iter().map(|(sid, _, _, _)| *sid).collect();

        for (_, name, x, y) in &member_infos {
            let pkt = SSendMemberLocation {
                member_name: name.clone(),
                member_location_x: *x,
                member_location_y: *y,
            };
            if let Ok(raw) = pkt.encode() {
                let encoded = Self::encode_raw(raw);
                for sid in &sids {
                    if *sid == self.session_id {
                        out.push(encoded.clone());
                    } else {
                        outboxes.entry(*sid).or_default().push(encoded.clone());
                    }
                }
            }
        }
    }
}

