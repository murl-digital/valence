#![allow(clippy::type_complexity)]

use valence::client::despawn_disconnected_clients;
use valence::client::event::{
    default_event_handler, PlayerInteractBlock, StartDigging, StartSneaking, StopDestroyBlock,
};
use valence::prelude::*;
use valence::protocol::types::Hand;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_systems(
            (
                default_event_handler,
                toggle_gamemode_on_sneak,
                digging_creative_mode,
                digging_survival_mode,
                place_blocks,
            )
                .in_schedule(EventLoopSchedule),
        )
        .add_systems(PlayerList::default_systems())
        .run();
}

fn setup(mut commands: Commands, server: Res<Server>) {
    let mut instance = server.new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<
        (
            Entity,
            &UniqueId,
            &mut Client,
            &mut Position,
            &mut Location,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, mut client, mut pos, mut loc, mut game_mode) in &mut clients {
        pos.0 = [0.0, SPAWN_Y as f64 + 1.0, 0.0].into();
        loc.0 = instances.single();
        *game_mode = GameMode::Creative;
        client.send_message("Welcome to Valence! Build something cool.".italic());
        commands
            .entity(entity)
            .insert(McEntity::with_uuid(EntityKind::Player, loc.0, uuid.0));
    }
}

fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut GameMode>,
    mut events: EventReader<StartSneaking>,
) {
    for event in events.iter() {
        let Ok(mut mode) = clients.get_component_mut::<GameMode>(event.client) else {
            continue;
        };
        *mode = match *mode {
            GameMode::Survival => GameMode::Creative,
            GameMode::Creative => GameMode::Survival,
            _ => GameMode::Creative,
        };
    }
}

fn digging_creative_mode(
    clients: Query<&GameMode>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<StartDigging>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok(game_mode) = clients.get(event.client) else {
            continue;
        };
        if *game_mode == GameMode::Creative {
            instance.set_block(event.position, BlockState::AIR);
        }
    }
}

fn digging_survival_mode(
    clients: Query<&GameMode>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<StopDestroyBlock>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok(game_mode) = clients.get(event.client) else {
            continue;
        };
        if *game_mode == GameMode::Survival {
            instance.set_block(event.position, BlockState::AIR);
        }
    }
}

fn place_blocks(
    mut clients: Query<(&mut Inventory, &GameMode, &PlayerInventoryState)>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<PlayerInteractBlock>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok((mut inventory, game_mode, inv_state)) = clients.get_mut(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        // get the held item
        let slot_id = inv_state.held_item_slot();
        let Some(stack) = inventory.slot(slot_id) else {
            // no item in the slot
            continue;
        };

        let Some(block_kind) = stack.item.to_block_kind() else {
            // can't place this item as a block
            continue;
        };

        if *game_mode == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            if stack.count() > 1 {
                let count = stack.count();
                inventory.set_slot_amount(slot_id, count - 1);
            } else {
                inventory.set_slot(slot_id, None);
            }
        }
        let real_pos = event.position.get_in_direction(event.direction);
        instance.set_block(real_pos, block_kind.to_state());
    }
}
