use std::sync::atomic::Ordering;
use std::time::Instant;

use crystal_server_core::account::CharacterPosition;
use crystal_server_net::ConnectionHandler;
use crystal_shared_proto::guild::{
    CEditGuildMember,
    CEditGuildNotice,
    CRequestGuildInfo,
    CGuildInvite,
    CGuildNameReturn,
    CGuildStorageGoldChange,
    CGuildStorageItemChange,
};
use crystal_shared_proto::item::{CDropItem, CStoreItem, CTakeBackItem, CSplitItem, CMergeItem, CDropGold, CRemoveSlotItem, CBuyItemBack, CEquipSlotItem, CCombineItem};
use crystal_shared_proto::npc::{CBuyItem, CCraftItem, CDepositTradeItem, CRetrieveTradeItem, CRepairItem, CSRepairItem, CDepositRefineItem, CRetrieveRefineItem, CRefineItem, CCheckRefine, CReplaceWedRing};
use crystal_shared_proto::login::CRefineCancel;
#[allow(unused_imports)]
use crystal_shared_proto::login::{
    CAbandonQuest,
    CAcceptQuest,
    CAcceptReincarnation,
    CAddFriend,
    CAddMember,
    CAddMemo,
    CAttack,
    CCallNPC,
    CChangeAMode,
    CChangePMode,
    CChangeTrade,
    CHarvest,
    CFishingCast,
    CFishingChangeAutocast,
    CAwakeningNeedMaterials,
    CAwakeningLockedItem,
    CAwakening,
    CDisassembleItem,
    CDowngradeAwakening,
    CResetAddedItem,
    CRequestIntelligentCreatureUpdates,
    CUpdateIntelligentCreature,
    CIntelligentCreaturePickup,
    CMarriageRequest,
    CMarriageReply,
    CChangeMarriage,
    CDivorceRequest,
    CDivorceReply,
    CAddMentor,
    CMentorReply,
    CAllowMentor,
    CCancelMentor,
    CGuildBuffUpdate,
    CNPCConfirmInput,
    CReportIssue,
    COpendoor,
    CGetRentedItems,
    CItemRentalRequest,
    CItemRentalFee,
    CItemRentalPeriod,
    CDepositRentalItem,
    CRetrieveRentalItem,
    CCancelItemRental,
    CItemRentalLockFee,
    CItemRentalLockItem,
    CConfirmItemRental,
    CGuildTerritoryPage,
    CPurchaseGuildTerritory,
    CConsignItem,
    CMarketSearch,
    CMarketRefresh,
    CMarketPage,
    CMarketBuy,
    CMarketSellNow,
    CMarketGetBack,
    CChangePassword,
    CGuildWarReturn,
    CClientVersion,
    CCollectParcel,
    CChat,
    CDeleteCharacter,
    CDeleteMail,
    CDelMember,
    CEquipItem,
    CFinishQuest,
    CKeepAlive,
    CLockMail,
    CLogin,
    CMailCost,
    CMailLockedItem,
    CMagic,
    CMagicKey,
    CMoveItem,
    CNewHero,
    CNewAccount,
    CNewCharacter,
    CPickUp,
    CRangeAttack,
    CReadMail,
    CRefreshFriends,
    CRemoveFriend,
    CRemoveItem,
    CRequestMapInfo,
    CSearchMap,
    CRun,
    CSendMail,
    CShareQuest,
    CSpellToggle,
    CStartGame,
    CSwitchGroup,
    CGroupInvite,
    CTeleportToNPC,
    CTownRevive,
    CTradeCancel,
    CTradeConfirm,
    CTradeGold,
    CTradeReply,
    CTradeRequest,
    CSetAutoPotItem,
    CSetAutoPotValue,
    CSetHeroBehaviour,
    CChangeHero,
    CTakeBackHeroItem,
    CTransferHeroItem,
    CTurn,
    CUseItem,
    CWalk,
    CCancelReincarnation,
    CGameshopBuy,
    CInspect,
    CObserve,
    CRequestUserName,
    CRequestChatItem,
    ClientPacketId,
    SConnected,
};
use crystal_shared_proto::ranking::CGetRanking;
use crystal_shared_proto::user::group::{SDeleteGroup, SDeleteMember};
use crystal_shared_proto::packet::RawPacket;

use super::{LoginConnection, Stage};

impl ConnectionHandler for LoginConnection {
    fn on_connect(&mut self) -> Vec<Vec<u8>> {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        vec![Self::encode_raw(SConnected.encode())]
    }

    fn handle_packet(&mut self, packet: RawPacket) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        // Temporary debug: see whether we ever receive raw Buy/Sell item
        // packets from the client.
        if packet.id == ClientPacketId::BuyItem as i16 {
            tracing::debug!(
                "[ingame] raw BuyItem packet received: id={} payload_len={}",
                packet.id,
                packet.payload.len(),
            );
        } else if packet.id == ClientPacketId::SellItem as i16 {
            tracing::debug!(
                "[ingame] raw SellItem packet received: id={} payload_len={}",
                packet.id,
                packet.payload.len(),
            );
        }

        let Some(pid) = ClientPacketId::from_i16(packet.id) else {
            // Temporary debug: surface any packet IDs that are not mapped in
            // ClientPacketId so we can see what the client is actually
            // sending for operations like NPC Buy.
            tracing::debug!(
                "[ingame] unknown client packet id={} payload_len={}",
                packet.id,
                packet.payload.len(),
            );
            return out;
        };

        self.last_active = Instant::now();

        match pid {
            ClientPacketId::NewAccount => {
                if let Ok(msg) = CNewAccount::decode(&packet.payload) {
                    self.handle_new_account(msg, &mut out);
                }
            }
            ClientPacketId::CraftItem => {
                match CCraftItem::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_craft_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CCraftItem from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::RepairItem => {
                if let Ok(msg) = CRepairItem::decode(&packet.payload) {
                    self.handle_repair_item(msg, &mut out);
                }
            }
            ClientPacketId::SRepairItem => {
                if let Ok(msg) = CSRepairItem::decode(&packet.payload) {
                    self.handle_srepair_item(msg, &mut out);
                }
            }
            ClientPacketId::ClientVersion => {
                if let Ok(msg) = CClientVersion::decode(&packet.payload) {
                    self.handle_client_version(msg, &mut out);
                }
            }
            ClientPacketId::Login => {
                if let Ok(msg) = CLogin::decode(&packet.payload) {
                    self.handle_login(msg, &mut out);
                }
            }
            ClientPacketId::ChangePassword => {
                if let Ok(msg) = CChangePassword::decode(&packet.payload) {
                    self.handle_change_password(msg, &mut out);
                }
            }
            ClientPacketId::NewCharacter => {
                if let Ok(msg) = CNewCharacter::decode(&packet.payload) {
                    self.handle_new_character(msg, &mut out);
                }
            }
            ClientPacketId::DeleteCharacter => {
                if let Ok(msg) = CDeleteCharacter::decode(&packet.payload) {
                    self.handle_delete_character(msg, &mut out);
                }
            }
            ClientPacketId::StartGame => {
                if let Ok(msg) = CStartGame::decode(&packet.payload) {
                    self.handle_start_game(msg, &mut out);
                }
            }
            ClientPacketId::LogOut => {
                self.handle_log_out(&mut out);
            }
            ClientPacketId::Turn => {
                if let Ok(msg) = CTurn::decode(&packet.payload) {
                    self.handle_turn(msg, &mut out);
                }
            }
            ClientPacketId::Walk => {
                if let Ok(msg) = CWalk::decode(&packet.payload) {
                    self.handle_walk(msg, &mut out);
                }
            }
            ClientPacketId::Run => {
                if let Ok(msg) = CRun::decode(&packet.payload) {
                    self.handle_run(msg, &mut out);
                }
            }
            ClientPacketId::Chat => {
                if let Ok(msg) = CChat::decode(&packet.payload) {
                    self.handle_chat(msg, &mut out);
                }
            }
            ClientPacketId::MoveItem => {
                if let Ok(msg) = CMoveItem::decode(&packet.payload) {
                    self.handle_move_item(msg, &mut out);
                }
            }
            ClientPacketId::StoreItem => {
                if let Ok(msg) = CStoreItem::decode(&packet.payload) {
                    self.handle_store_item(msg, &mut out);
                }
            }
            ClientPacketId::TakeBackItem => {
                if let Ok(msg) = CTakeBackItem::decode(&packet.payload) {
                    self.handle_take_back_item(msg, &mut out);
                }
            }
            ClientPacketId::UseItem => {
                if let Ok(msg) = CUseItem::decode(&packet.payload) {
                    self.handle_use_item(msg, &mut out);
                }
            }
            ClientPacketId::DropItem => {
                if let Ok(msg) = CDropItem::decode(&packet.payload) {
                    self.handle_drop_item(msg, &mut out);
                }
            }
            ClientPacketId::MergeItem => {
                if let Ok(msg) = CMergeItem::decode(&packet.payload) {
                    self.handle_merge_item(msg, &mut out);
                }
            }
            ClientPacketId::SplitItem => {
                if let Ok(msg) = CSplitItem::decode(&packet.payload) {
                    self.handle_split_item(msg, &mut out);
                }
            }
            ClientPacketId::DropGold => {
                if let Ok(msg) = CDropGold::decode(&packet.payload) {
                    self.handle_drop_gold(msg, &mut out);
                }
            }
            ClientPacketId::RemoveSlotItem => {
                if let Ok(msg) = CRemoveSlotItem::decode(&packet.payload) {
                    self.handle_remove_slot_item(msg, &mut out);
                }
            }
            ClientPacketId::BuyItemBack => {
                if let Ok(msg) = CBuyItemBack::decode(&packet.payload) {
                    self.handle_buy_item_back(msg, &mut out);
                }
            }
            ClientPacketId::DepositRefineItem => {
                if let Ok(msg) = CDepositRefineItem::decode(&packet.payload) {
                    self.handle_deposit_refine_item(msg, &mut out);
                }
            }
            ClientPacketId::RetrieveRefineItem => {
                if let Ok(msg) = CRetrieveRefineItem::decode(&packet.payload) {
                    self.handle_retrieve_refine_item(msg, &mut out);
                }
            }
            ClientPacketId::RefineCancel => {
                if let Ok(_msg) = CRefineCancel::decode(&packet.payload) {
                    self.handle_refine_cancel(&mut out);
                }
            }
            ClientPacketId::RefineItem => {
                if let Ok(msg) = CRefineItem::decode(&packet.payload) {
                    self.handle_refine_item(msg, &mut out);
                }
            }
            ClientPacketId::CheckRefine => {
                if let Ok(msg) = CCheckRefine::decode(&packet.payload) {
                    self.handle_check_refine(msg, &mut out);
                }
            }
            ClientPacketId::AwakeningNeedMaterials => {
                if let Ok(msg) = CAwakeningNeedMaterials::decode(&packet.payload) {
                    self.handle_awakening_need_materials(msg, &mut out);
                }
            }
            ClientPacketId::AwakeningLockedItem => {
                if let Ok(msg) = CAwakeningLockedItem::decode(&packet.payload) {
                    self.handle_awakening_locked_item(msg, &mut out);
                }
            }
            ClientPacketId::Awakening => {
                if let Ok(msg) = CAwakening::decode(&packet.payload) {
                    self.handle_awakening(msg, &mut out);
                }
            }
            ClientPacketId::DisassembleItem => {
                if let Ok(msg) = CDisassembleItem::decode(&packet.payload) {
                    self.handle_disassemble_item(msg, &mut out);
                }
            }
            ClientPacketId::DowngradeAwakening => {
                if let Ok(msg) = CDowngradeAwakening::decode(&packet.payload) {
                    self.handle_downgrade_awakening(msg, &mut out);
                }
            }
            ClientPacketId::ResetAddedItem => {
                if let Ok(msg) = CResetAddedItem::decode(&packet.payload) {
                    self.handle_reset_added_item(msg, &mut out);
                }
            }
            ClientPacketId::ReplaceWedRing => {
                if let Ok(msg) = CReplaceWedRing::decode(&packet.payload) {
                    self.handle_replace_wed_ring(msg, &mut out);
                }
            }
            ClientPacketId::EquipSlotItem => {
                if let Ok(msg) = CEquipSlotItem::decode(&packet.payload) {
                    self.handle_equip_slot_item(msg, &mut out);
                }
            }
            ClientPacketId::CombineItem => {
                if let Ok(msg) = CCombineItem::decode(&packet.payload) {
                    self.handle_combine_item(msg, &mut out);
                }
            }
            ClientPacketId::TakeBackHeroItem => {
                if let Ok(msg) = CTakeBackHeroItem::decode(&packet.payload) {
                    self.handle_take_back_hero_item(msg, &mut out);
                }
            }
            ClientPacketId::TransferHeroItem => {
                if let Ok(msg) = CTransferHeroItem::decode(&packet.payload) {
                    self.handle_transfer_hero_item(msg, &mut out);
                }
            }
            ClientPacketId::BuyItem => {
                tracing::debug!(
                    "[ingame] dispatch BuyItem: id={} payload_len={}",
                    packet.id,
                    packet.payload.len(),
                );
                match CBuyItem::decode(&packet.payload) {
                    Ok(msg) => {
                        tracing::debug!(
                            "[ingame] decoded BuyItem: item_index={} count={} panel_type={}",
                            msg.item_index,
                            msg.count,
                            msg.panel_type,
                        );
                        self.handle_buy_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "[ingame] failed to decode CBuyItem: {:?}, payload_len={}",
                            e,
                            packet.payload.len(),
                        );
                    }
                }
            }
            ClientPacketId::SellItem => {
                tracing::debug!(
                    "[ingame] dispatch SellItem: id={} payload_len={}",
                    packet.id,
                    packet.payload.len(),
                );
                match crystal_shared_proto::npc::CSellItem::decode(&packet.payload) {
                    Ok(msg) => {
                        tracing::debug!(
                            "[ingame] decoded SellItem: unique_id={} count={}",
                            msg.unique_id,
                            msg.count,
                        );
                        self.handle_sell_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "[ingame] failed to decode CSellItem: {:?}, payload_len={}",
                            e,
                            packet.payload.len(),
                        );
                    }
                }
            }
            ClientPacketId::EquipItem => {
                if let Ok(msg) = CEquipItem::decode(&packet.payload) {
                    self.handle_equip_item(msg, &mut out);
                }
            }
            ClientPacketId::RemoveItem => {
                if let Ok(msg) = CRemoveItem::decode(&packet.payload) {
                    self.handle_remove_item(msg, &mut out);
                }
            }
            ClientPacketId::CallNPC => {
                if let Ok(msg) = CCallNPC::decode(&packet.payload) {
                    self.handle_call_npc(msg, &mut out);
                }
            }
            ClientPacketId::MagicKey => {
                if let Ok(msg) = CMagicKey::decode(&packet.payload) {
                    self.handle_magic_key(msg, &mut out);
                }
            }
            ClientPacketId::ChangeAMode => {
                if let Ok(msg) = CChangeAMode::decode(&packet.payload) {
                    self.handle_change_attack_mode(msg, &mut out);
                }
            }
            ClientPacketId::ChangePMode => {
                if let Ok(msg) = CChangePMode::decode(&packet.payload) {
                    self.handle_change_pet_mode(msg, &mut out);
                }
            }
            ClientPacketId::ChangeTrade => {
                if let Ok(msg) = CChangeTrade::decode(&packet.payload) {
                    self.handle_change_trade(msg, &mut out);
                }
            }
            ClientPacketId::Magic => {
                if let Ok(msg) = CMagic::decode(&packet.payload) {
                    self.handle_magic(msg, &mut out);
                }
            }
            ClientPacketId::Attack => {
                if let Ok(msg) = CAttack::decode(&packet.payload) {
                    self.handle_attack(msg, &mut out);
                }
            }
            ClientPacketId::RangeAttack => {
                if let Ok(msg) = CRangeAttack::decode(&packet.payload) {
                    self.handle_range_attack(msg, &mut out);
                }
            }
            ClientPacketId::SpellToggle => {
                if let Ok(msg) = CSpellToggle::decode(&packet.payload) {
                    self.handle_spell_toggle(msg, &mut out);
                }
            }
            ClientPacketId::FishingCast => {
                if let Ok(msg) = CFishingCast::decode(&packet.payload) {
                    self.handle_fishing_cast(msg, &mut out);
                }
            }
            ClientPacketId::FishingChangeAutocast => {
                if let Ok(msg) = CFishingChangeAutocast::decode(&packet.payload) {
                    self.handle_fishing_change_autocast(msg, &mut out);
                }
            }
            ClientPacketId::AwakeningNeedMaterials => {
                if let Ok(msg) = CAwakeningNeedMaterials::decode(&packet.payload) {
                    self.handle_awakening_need_materials(msg, &mut out);
                }
            }
            ClientPacketId::AwakeningLockedItem => {
                if let Ok(msg) = CAwakeningLockedItem::decode(&packet.payload) {
                    self.handle_awakening_locked_item(msg, &mut out);
                }
            }
            ClientPacketId::Awakening => {
                if let Ok(msg) = CAwakening::decode(&packet.payload) {
                    self.handle_awakening(msg, &mut out);
                }
            }
            ClientPacketId::DisassembleItem => {
                if let Ok(msg) = CDisassembleItem::decode(&packet.payload) {
                    self.handle_disassemble_item(msg, &mut out);
                }
            }
            ClientPacketId::DowngradeAwakening => {
                if let Ok(msg) = CDowngradeAwakening::decode(&packet.payload) {
                    self.handle_downgrade_awakening(msg, &mut out);
                }
            }
            ClientPacketId::ResetAddedItem => {
                if let Ok(msg) = CResetAddedItem::decode(&packet.payload) {
                    self.handle_reset_added_item(msg, &mut out);
                }
            }
            ClientPacketId::RequestIntelligentCreatureUpdates => {
                if let Ok(msg) = CRequestIntelligentCreatureUpdates::decode(&packet.payload) {
                    self.handle_request_intelligent_creature_updates(msg, &mut out);
                }
            }
            ClientPacketId::UpdateIntelligentCreature => {
                if let Ok(msg) = CUpdateIntelligentCreature::decode(&packet.payload) {
                    self.handle_update_intelligent_creature(msg, &mut out);
                }
            }
            ClientPacketId::IntelligentCreaturePickup => {
                if let Ok(msg) = CIntelligentCreaturePickup::decode(&packet.payload) {
                    self.handle_intelligent_creature_pickup(msg, &mut out);
                }
            }
            ClientPacketId::MarriageRequest => {
                if let Ok(_msg) = CMarriageRequest::decode(&packet.payload) {
                    self.handle_marriage_request(&mut out);
                }
            }
            ClientPacketId::MarriageReply => {
                if let Ok(msg) = CMarriageReply::decode(&packet.payload) {
                    self.handle_marriage_reply(msg, &mut out);
                }
            }
            ClientPacketId::ChangeMarriage => {
                if let Ok(_msg) = CChangeMarriage::decode(&packet.payload) {
                    self.handle_change_marriage(&mut out);
                }
            }
            ClientPacketId::DivorceRequest => {
                if let Ok(_msg) = CDivorceRequest::decode(&packet.payload) {
                    self.handle_divorce_request(&mut out);
                }
            }
            ClientPacketId::DivorceReply => {
                if let Ok(msg) = CDivorceReply::decode(&packet.payload) {
                    self.handle_divorce_reply(msg, &mut out);
                }
            }
            ClientPacketId::AddMentor => {
                if let Ok(msg) = CAddMentor::decode(&packet.payload) {
                    self.handle_add_mentor(msg, &mut out);
                }
            }
            ClientPacketId::MentorReply => {
                if let Ok(msg) = CMentorReply::decode(&packet.payload) {
                    self.handle_mentor_reply(msg, &mut out);
                }
            }
            ClientPacketId::AllowMentor => {
                if let Ok(_msg) = CAllowMentor::decode(&packet.payload) {
                    self.handle_allow_mentor(&mut out);
                }
            }
            ClientPacketId::CancelMentor => {
                if let Ok(_msg) = CCancelMentor::decode(&packet.payload) {
                    self.handle_cancel_mentor(&mut out);
                }
            }
            ClientPacketId::GuildBuffUpdate => {
                if let Ok(msg) = CGuildBuffUpdate::decode(&packet.payload) {
                    self.handle_guild_buff_update(msg, &mut out);
                }
            }
            ClientPacketId::NPCConfirmInput => {
                if let Ok(msg) = CNPCConfirmInput::decode(&packet.payload) {
                    self.handle_npc_confirm_input(msg, &mut out);
                }
            }
            ClientPacketId::ReportIssue => {
                if let Ok(msg) = CReportIssue::decode(&packet.payload) {
                    self.handle_report_issue(msg, &mut out);
                }
            }
            ClientPacketId::Opendoor => {
                if let Ok(msg) = COpendoor::decode(&packet.payload) {
                    self.handle_opendoor(msg, &mut out);
                }
            }
            ClientPacketId::GetRentedItems => {
                if let Ok(_msg) = CGetRentedItems::decode(&packet.payload) {
                    self.handle_get_rented_items(&mut out);
                }
            }
            ClientPacketId::ItemRentalRequest => {
                if let Ok(_msg) = CItemRentalRequest::decode(&packet.payload) {
                    self.handle_item_rental_request(&mut out);
                }
            }
            ClientPacketId::ItemRentalFee => {
                if let Ok(msg) = CItemRentalFee::decode(&packet.payload) {
                    self.handle_item_rental_fee(msg, &mut out);
                }
            }
            ClientPacketId::ItemRentalPeriod => {
                if let Ok(msg) = CItemRentalPeriod::decode(&packet.payload) {
                    self.handle_item_rental_period(msg, &mut out);
                }
            }
            ClientPacketId::DepositRentalItem => {
                if let Ok(msg) = CDepositRentalItem::decode(&packet.payload) {
                    self.handle_deposit_rental_item(msg, &mut out);
                }
            }
            ClientPacketId::RetrieveRentalItem => {
                if let Ok(msg) = CRetrieveRentalItem::decode(&packet.payload) {
                    self.handle_retrieve_rental_item(msg, &mut out);
                }
            }
            ClientPacketId::CancelItemRental => {
                if let Ok(_msg) = CCancelItemRental::decode(&packet.payload) {
                    self.handle_cancel_item_rental(&mut out);
                }
            }
            ClientPacketId::ItemRentalLockFee => {
                if let Ok(_msg) = CItemRentalLockFee::decode(&packet.payload) {
                    self.handle_item_rental_lock_fee(&mut out);
                }
            }
            ClientPacketId::ItemRentalLockItem => {
                if let Ok(_msg) = CItemRentalLockItem::decode(&packet.payload) {
                    self.handle_item_rental_lock_item(&mut out);
                }
            }
            ClientPacketId::ConfirmItemRental => {
                if let Ok(_msg) = CConfirmItemRental::decode(&packet.payload) {
                    self.handle_confirm_item_rental(&mut out);
                }
            }
            ClientPacketId::GuildTerritoryPage => {
                if let Ok(msg) = CGuildTerritoryPage::decode(&packet.payload) {
                    self.handle_guild_territory_page(msg, &mut out);
                }
            }
            ClientPacketId::PurchaseGuildTerritory => {
                if let Ok(msg) = CPurchaseGuildTerritory::decode(&packet.payload) {
                    self.handle_purchase_guild_territory(msg, &mut out);
                }
            }
            ClientPacketId::PickUp => {
                if let Ok(msg) = CPickUp::decode(&packet.payload) {
                    self.handle_pick_up(msg, &mut out);
                }
            }
            ClientPacketId::RequestMapInfo => {
                if let Ok(msg) = CRequestMapInfo::decode(&packet.payload) {
                    self.handle_request_map_info(msg, &mut out);
                }
            }
            ClientPacketId::GetRanking => {
                if let Ok(msg) = CGetRanking::decode(&packet.payload) {
                    self.handle_get_ranking(msg, &mut out);
                }
            }
            ClientPacketId::TeleportToNPC => {
                if let Ok(msg) = CTeleportToNPC::decode(&packet.payload) {
                    self.handle_teleport_to_npc(msg, &mut out);
                }
            }
            ClientPacketId::SearchMap => {
                if let Ok(msg) = CSearchMap::decode(&packet.payload) {
                    self.handle_search_map(msg, &mut out);
                }
            }
            ClientPacketId::Inspect => {
                if let Ok(msg) = CInspect::decode(&packet.payload) {
                    self.handle_inspect(msg, &mut out);
                }
            }
            ClientPacketId::Observe => {
                if let Ok(msg) = CObserve::decode(&packet.payload) {
                    self.handle_observe(msg, &mut out);
                }
            }
            ClientPacketId::EditGuildMember => {
                if let Ok(msg) = CEditGuildMember::decode(&packet.payload) {
                    self.handle_edit_guild_member(msg, &mut out);
                }
            }
            ClientPacketId::EditGuildNotice => {
                if let Ok(msg) = CEditGuildNotice::decode(&packet.payload) {
                    self.handle_edit_guild_notice(msg, &mut out);
                }
            }
            ClientPacketId::TownRevive => {
                if let Ok(msg) = CTownRevive::decode(&packet.payload) {
                    self.handle_town_revive(msg, &mut out);
                }
            }
            ClientPacketId::GuildInvite => {
                if let Ok(msg) = CGuildInvite::decode(&packet.payload) {
                    self.handle_guild_invite(msg, &mut out);
                }
            }
            ClientPacketId::GuildNameReturn => {
                if let Ok(msg) = CGuildNameReturn::decode(&packet.payload) {
                    self.handle_guild_name_return(msg, &mut out);
                }
            }
            ClientPacketId::RequestGuildInfo => {
                if let Ok(msg) = CRequestGuildInfo::decode(&packet.payload) {
                    self.handle_request_guild_info(msg, &mut out);
                }
            }
            ClientPacketId::GuildStorageGoldChange => {
                match CGuildStorageGoldChange::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_guild_storage_gold_change(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CGuildStorageGoldChange from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::GuildStorageItemChange => {
                match CGuildStorageItemChange::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_guild_storage_item_change(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CGuildStorageItemChange from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::GuildWarReturn => {
                if let Ok(msg) = CGuildWarReturn::decode(&packet.payload) {
                    self.handle_guild_war_return(msg, &mut out);
                }
            }
            ClientPacketId::KeepAlive => {
                if let Ok(msg) = CKeepAlive::decode(&packet.payload) {
                    self.handle_keep_alive(msg, &mut out);
                }
            }
            ClientPacketId::Disconnect => {
                self.closing = true;
            }
            ClientPacketId::Harvest => {
                if let Ok(msg) = CHarvest::decode(&packet.payload) {
                    self.handle_harvest(msg, &mut out);
                }
            }
            // Trade protocols: request and reply are now partially handled.
            ClientPacketId::TradeRequest => {
                match CTradeRequest::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_trade_request(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeRequest from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::TradeReply => {
                match CTradeReply::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_trade_reply(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeReply from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::TradeGold => {
                match CTradeGold::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_trade_gold(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeGold from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::TradeConfirm => {
                match CTradeConfirm::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_trade_confirm(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeConfirm from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::TradeCancel => {
                match CTradeCancel::decode(&packet.payload) {
                    Ok(_) => {
                        self.handle_trade_cancel(&mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeCancel from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::ConsignItem => {
                if let Ok(msg) = CConsignItem::decode(&packet.payload) {
                    self.handle_consign_item(msg, &mut out);
                }
            }
            ClientPacketId::MarketSearch => {
                if let Ok(msg) = CMarketSearch::decode(&packet.payload) {
                    self.handle_market_search(msg, &mut out);
                }
            }
            ClientPacketId::MarketRefresh => {
                if let Ok(_msg) = CMarketRefresh::decode(&packet.payload) {
                    self.handle_market_refresh(&mut out);
                }
            }
            ClientPacketId::MarketPage => {
                if let Ok(msg) = CMarketPage::decode(&packet.payload) {
                    self.handle_market_page(msg, &mut out);
                }
            }
            ClientPacketId::MarketBuy => {
                if let Ok(msg) = CMarketBuy::decode(&packet.payload) {
                    self.handle_market_buy(msg, &mut out);
                }
            }
            ClientPacketId::MarketGetBack => {
                if let Ok(msg) = CMarketGetBack::decode(&packet.payload) {
                    self.handle_market_get_back(msg, &mut out);
                }
            }
            ClientPacketId::MarketSellNow => {
                if let Ok(msg) = CMarketSellNow::decode(&packet.payload) {
                    self.handle_market_sell_now(msg, &mut out);
                }
            }
            ClientPacketId::RequestUserName => {
                if let Ok(msg) = CRequestUserName::decode(&packet.payload) {
                    self.handle_request_user_name(msg, &mut out);
                }
            }
            ClientPacketId::RequestChatItem => {
                if let Ok(msg) = CRequestChatItem::decode(&packet.payload) {
                    self.handle_request_chat_item(msg, &mut out);
                }
            }
            ClientPacketId::DepositTradeItem => {
                match CDepositTradeItem::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_deposit_trade_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CDepositTradeItem from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::RetrieveTradeItem => {
                match CRetrieveTradeItem::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_retrieve_trade_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CRetrieveTradeItem from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            // Quest protocols
            ClientPacketId::AcceptQuest => {
                if let Ok(msg) = CAcceptQuest::decode(&packet.payload) {
                    self.handle_accept_quest(msg, &mut out);
                }
            }
            ClientPacketId::FinishQuest => {
                if let Ok(msg) = CFinishQuest::decode(&packet.payload) {
                    self.handle_finish_quest(msg, &mut out);
                }
            }
            ClientPacketId::AbandonQuest => {
                if let Ok(msg) = CAbandonQuest::decode(&packet.payload) {
                    self.handle_abandon_quest(msg, &mut out);
                }
            }
            ClientPacketId::ShareQuest => {
                if let Ok(msg) = CShareQuest::decode(&packet.payload) {
                    self.handle_share_quest(msg, &mut out);
                }
            }
            ClientPacketId::AcceptReincarnation => {
                if let Ok(_) = CAcceptReincarnation::decode(&packet.payload) {
                    self.handle_accept_reincarnation(&mut out);
                }
            }
            ClientPacketId::CancelReincarnation => {
                if let Ok(_) = CCancelReincarnation::decode(&packet.payload) {
                    self.handle_cancel_reincarnation(&mut out);
                }
            }
            // Group protocols (stub - not implemented yet)
            ClientPacketId::SwitchGroup => {
                if let Ok(msg) = CSwitchGroup::decode(&packet.payload) {
                    self.handle_switch_group(msg, &mut out);
                }
            }
            ClientPacketId::AddMember => {
                if let Ok(msg) = CAddMember::decode(&packet.payload) {
                    self.handle_add_member(msg, &mut out);
                }
            }
            ClientPacketId::DellMember => {
                if let Ok(msg) = CDelMember::decode(&packet.payload) {
                    self.handle_del_member(msg, &mut out);
                }
            }
            ClientPacketId::GroupInvite => {
                if let Ok(msg) = CGroupInvite::decode(&packet.payload) {
                    self.handle_group_invite(msg, &mut out);
                }
            }
            ClientPacketId::NewHero => {
                if let Ok(msg) = CNewHero::decode(&packet.payload) {
                    self.handle_new_hero(msg, &mut out);
                }
            }
            ClientPacketId::SetAutoPotValue => {
                if let Ok(msg) = CSetAutoPotValue::decode(&packet.payload) {
                    self.handle_set_auto_pot_value(msg, &mut out);
                }
            }
            ClientPacketId::SetAutoPotItem => {
                if let Ok(msg) = CSetAutoPotItem::decode(&packet.payload) {
                    self.handle_set_auto_pot_item(msg, &mut out);
                }
            }
            ClientPacketId::SetHeroBehaviour => {
                if let Ok(msg) = CSetHeroBehaviour::decode(&packet.payload) {
                    self.handle_set_hero_behaviour(msg, &mut out);
                }
            }
            ClientPacketId::ChangeHero => {
                if let Ok(msg) = CChangeHero::decode(&packet.payload) {
                    self.handle_change_hero(msg, &mut out);
                }
            }
            ClientPacketId::SendMail => {
                if let Ok(msg) = CSendMail::decode(&packet.payload) {
                    self.handle_send_mail(msg, &mut out);
                }
            }
            ClientPacketId::ReadMail => {
                if let Ok(msg) = CReadMail::decode(&packet.payload) {
                    self.handle_read_mail(msg, &mut out);
                }
            }
            ClientPacketId::CollectParcel => {
                if let Ok(msg) = CCollectParcel::decode(&packet.payload) {
                    self.handle_collect_parcel(msg, &mut out);
                }
            }
            ClientPacketId::DeleteMail => {
                if let Ok(msg) = CDeleteMail::decode(&packet.payload) {
                    self.handle_delete_mail(msg, &mut out);
                }
            }
            ClientPacketId::LockMail => {
                if let Ok(msg) = CLockMail::decode(&packet.payload) {
                    self.handle_lock_mail(msg, &mut out);
                }
            }
            ClientPacketId::MailLockedItem => {
                if let Ok(msg) = CMailLockedItem::decode(&packet.payload) {
                    self.handle_mail_locked_item(msg, &mut out);
                }
            }
            ClientPacketId::MailCost => {
                if let Ok(msg) = CMailCost::decode(&packet.payload) {
                    self.handle_mail_cost(msg, &mut out);
                }
            }
            ClientPacketId::GameshopBuy => {
                match CGameshopBuy::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_gameshop_buy(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CGameshopBuy from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::AddFriend => {
                if let Ok(msg) = CAddFriend::decode(&packet.payload) {
                    self.handle_add_friend(msg, &mut out);
                }
            }
            ClientPacketId::RemoveFriend => {
                if let Ok(msg) = CRemoveFriend::decode(&packet.payload) {
                    self.handle_remove_friend(msg, &mut out);
                }
            }
            ClientPacketId::RefreshFriends => {
                if let Ok(msg) = CRefreshFriends::decode(&packet.payload) {
                    self.handle_refresh_friends(msg, &mut out);
                }
            }
            ClientPacketId::AddMemo => {
                if let Ok(msg) = CAddMemo::decode(&packet.payload) {
                    self.handle_add_memo(msg, &mut out);
                }
            }
            _ => {
                tracing::debug!(
                    "Unhandled client packet {:?} (id={} payload_len={})",
                    pid,
                    packet.id,
                    packet.payload.len(),
                );
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "该客户端操作尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
        }

        out
    }

    fn poll_outbound(&mut self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();

        if self.stage == Stage::InGame && self.current_map_index != 0 {
            self.update_visibility(&mut out);
        }

        let mut outboxes = self.outboxes.lock().unwrap();
        if let Some(mut queued) = outboxes.remove(&self.session_id) {
            out.append(&mut queued);
        }

        if !self.closing {
            let elapsed = self.last_active.elapsed();
            if elapsed.as_millis() as u64 > self.timeout_ms {
                self.closing = true;
            }
        }

        out
    }

    fn on_disconnect(&mut self) {
        if self.stage == Stage::InGame {
            if let (Some(ref account_id), Some(char_idx)) =
                (self.account_id.as_ref(), self.current_char_index)
            {
                let magics = {
                    let world = self.world.lock().unwrap();
                    world.player_magics(self.session_id)
                };
                let _ = self
                    .store
                    .save_character_magics(account_id, char_idx, &magics);

                let (inventory, equipment) = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_items(self.session_id)
                        .unwrap_or((crystal_server_core::item::Inventory::new_default(), crystal_server_core::item::Equipment::new_default()))
                };
                let refine_slots = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_refine_slots(self.session_id)
                        .unwrap_or_else(|| vec![None; 16])
                };
                let _ = self
                    .store
                    .save_character_items(account_id, char_idx, &inventory, &equipment, &refine_slots);

                let pos = CharacterPosition {
                    map_index: self.current_map_index,
                    x: self.current_x,
                    y: self.current_y,
                    direction: self.direction,
                };
                let _ = self
                    .store
                    .save_character_position(account_id, char_idx, &pos);

                if let Some(ch) = self
                    .characters
                    .iter()
                    .find(|c| c.index == char_idx)
                {
                    let _ = self
                        .store
                        .update_character_level(account_id, char_idx, ch.level);
                }
            }

            // Snapshot current party membership so we can notify remaining
            // members after this player is removed from the world, mirroring
            // C# PlayerObject.LeaveGroup on despawn.
            let party_members_before = {
                let world = self.world.lock().unwrap();
                world.party_members_for_session(self.session_id)
            };

            // Remove the player from the world state and occupancy tracking so
            // that disconnected characters no longer block movement.
            {
                let mut world = self.world.lock().unwrap();
                world.remove_player_from_world(self.session_id);
            }

            if let Some(members_before) = party_members_before {
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
                            let encoded = super::LoginConnection::encode_raw(raw);
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
                        let encoded = super::LoginConnection::encode_raw(pkt.encode());
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

        if let Some(ref account_id) = self.account_id {
            let mut map = self.online_accounts.lock().unwrap();
            if let Some(cur) = map.get(account_id) {
                if *cur == self.session_id {
                    map.remove(account_id);
                }
            }
        }

        {
            let mut map = self.player_summaries.lock().unwrap();
            map.remove(&self.session_id);
        }

        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    fn should_close(&self) -> bool {
        self.closing
    }
}
