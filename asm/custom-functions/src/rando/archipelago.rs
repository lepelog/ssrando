//! archipelago item giving

use crate::{
    game::{
        actor::ActorBase__kill,
        actor_reference::ActorReference,
        events::EventManager,
        file_manager::{self, get_current_file, get_selected_file_num},
        item::AcItem,
        minigame::SpecialMinigameState,
        player,
        reloader::is_on_title_screen,
    },
    println,
    rando::{
        custom_actor::{spawn_archi_item_give, BaseActor},
        give_item_with_sceneflag,
        item_arc_loader::ItemArcLoader,
    },
    system::os::with_disabled_interrupts,
};

extern "C" {
    static mut ARCHIPELAGO_CONTEXT: ArchipelagoContext;
}

// this works as follows:
// we assume no partial writes for 4 byte messages
//

#[repr(C)]
pub struct ArchipelagoContext {
    give_item:                 u8,
    // set by remote client
    item_given_index:          u16,
    expected_item_given_index: u16,
}

pub fn reset_archipelago_context() {
    unsafe {
        ARCHIPELAGO_CONTEXT.give_item = u8::MAX;
        ARCHIPELAGO_CONTEXT.expected_item_given_index = u16::MAX;
        ARCHIPELAGO_CONTEXT.item_given_index = u16::MAX;
    }
}

#[link_section = "data"]
pub static mut AP_QUEUE: heapless::Deque<u8, 16> = heapless::Deque::new();

pub fn handle_archipelago_item_giving() {
    // only handle normal file 1 gameplay
    if is_on_title_screen() {
        return;
    }
    if get_selected_file_num() != 0 {
        return;
    }
    // first, we try to put the items in the queue
    // this makes it quicker to send multiple items while ensuring
    // the item is actually received
    unsafe {
        let ctx_give_item = ARCHIPELAGO_CONTEXT.give_item;
        if ctx_give_item != u8::MAX {
            // make sure this is the item we expected
            if ARCHIPELAGO_CONTEXT.expected_item_given_index != ARCHIPELAGO_CONTEXT.item_given_index
            {
                // some race condition happened, so don't queue the item
                println!("expected != actual!");
                ARCHIPELAGO_CONTEXT.give_item = u8::MAX;
            } else if AP_QUEUE.push_back(ctx_give_item).is_ok() {
                // the ap client can send the next item now
                // the index is incremented first to make sure items aren't sent twice
                with_disabled_interrupts(|| {
                    ARCHIPELAGO_CONTEXT.expected_item_given_index += 1;
                    ARCHIPELAGO_CONTEXT.give_item = u8::MAX;
                });
                println!("enqueued!");
            }
        }
    }
    if let Some(item_to_give) = unsafe { AP_QUEUE.front().copied() } {
        if !is_giving_archipelago_item() {
            println!("spawned item!");
            spawn_archi_item_give(item_to_give.into());
        }
    }
}

fn finish_item_giving() {
    // item has been given, persist
    set_saved_item_index(get_saved_item_index() + 1);
    // remove item from queue
    unsafe {
        AP_QUEUE.pop_front();
    }
}

pub fn get_saved_item_index() -> u16 {
    // always use FA, we might still be on the title screen
    // when starting a file
    unsafe { (*file_manager::get_ptr()).FA.scene_flags[6][0] }
}

pub fn set_saved_item_index(idx: u16) {
    unsafe {
        (*get_current_file()).scene_flags[6][0] = idx;
    }
}

#[no_mangle]
pub extern "C" fn reset_ap_from_save() {
    // happens after dying in a player reset area (for example sandship mast
    // sequence)
    // or on starting file

    unsafe {
        AP_QUEUE.clear();
        ARCHIPELAGO_CONTEXT.expected_item_given_index = get_saved_item_index();
    }
}

// the client will set "give_item":
// - give_item is FF, not on title screen, file 1
// give item according ti item_given_index

// when starting a file,

pub struct ArchipelagoItemGiver {
    item_ref:              ActorReference<AcItem>,
    item_id:               u16,
    arc_loader:            ItemArcLoader,
    is_give_success:       bool,
    item_event_kill_delay: u16,
}

impl ArchipelagoItemGiver {
    pub fn new(item_id: u16) -> Self {
        unsafe {
            // only one of these can exist at a time
            IS_GIVING_ARCHIPELAGO_ITEM = true;
        }
        Self {
            item_ref:              ActorReference::new(),
            item_id:               item_id as u16,
            arc_loader:            ItemArcLoader::for_item(item_id as u16),
            is_give_success:       false,
            item_event_kill_delay: 0,
        }
    }

    pub fn update(&mut self, base_actor: &mut BaseActor) {
        // when the ref is set, check if in presenting state
        if let Some(item) = self.item_ref.get() {
            // yes: item is in event state, which means giving state
            if (unsafe { item.as_ref() }.actor_event_flags & 1) != 0 {
                self.is_give_success = true;
            } else if EventManager::is_in_event() {
                // when in any other event, kill the item
                // we try again later after the event is done
                // this prevents this item being "given" by some npc event
                self.item_event_kill_delay = self.item_event_kill_delay.wrapping_add(1);
                println!(
                    "killing item due to event ({}/5)!!!",
                    self.item_event_kill_delay
                );
                // we can't kill the item on the first frame after spawning it
                // due to link dereferencing a null pointer in that case
                if self.item_event_kill_delay >= 5 {
                    unsafe {
                        ActorBase__kill(item.cast().as_ptr());
                    }
                    self.item_event_kill_delay = 0;
                }
            }
        } else {
            // if giving the item succeeded and it despawned, we're done
            if self.is_give_success {
                // mark item as given
                // increment actual item given counter in save file
                // pop item from queue
                finish_item_giving();
                base_actor.kill();
                return;
            }
            // maybe the item despawned without success
            // or we haven't given the item at all yet
            // make sure arc is loaded
            if self.arc_loader.is_loaded() {
                if SpecialMinigameState::StateNone.is_current()
                    && !EventManager::is_in_event()
                    && unsafe {
                        player::get_ptr()
                            .as_ref()
                            .is_some_and(|p| p.current_state <= 0xD)
                    }
                {
                    println!("archi spawning {}", self.item_id);
                    let item_actor =
                        give_item_with_sceneflag(self.item_id, u32::MAX, u32::MAX, 0xFF)
                            as *mut AcItem;
                    self.item_ref.link(item_actor);
                }
            }
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            IS_GIVING_ARCHIPELAGO_ITEM = false;
        }
    }
}

static mut IS_GIVING_ARCHIPELAGO_ITEM: bool = false;

pub fn is_giving_archipelago_item() -> bool {
    unsafe { IS_GIVING_ARCHIPELAGO_ITEM }
}
