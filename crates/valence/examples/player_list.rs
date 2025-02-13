#![allow(clippy::type_complexity)]

use rand::Rng;
use valence::client::event::default_event_handler;
use valence::player_list::Entry;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;
const PLAYER_UUID_1: Uuid = Uuid::from_u128(1);
const PLAYER_UUID_2: Uuid = Uuid::from_u128(2);

fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(default_event_handler.in_schedule(EventLoopSchedule))
        .add_systems(PlayerList::default_systems())
        .add_systems((
            init_clients,
            update_player_list,
            remove_disconnected_clients_from_player_list,
            despawn_disconnected_clients,
        ))
        .run();
}

fn setup(mut commands: Commands, server: Res<Server>, mut player_list: ResMut<PlayerList>) {
    let mut instance = server.new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::LIGHT_GRAY_WOOL);
        }
    }

    commands.spawn(instance);

    player_list.insert(
        PLAYER_UUID_1,
        PlayerListEntry::new().with_display_name(Some("persistent entry with no ping")),
    );
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut Position,
            &mut Location,
            &mut GameMode,
            &Username,
            &Properties,
            &UniqueId,
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
    mut player_list: ResMut<PlayerList>,
) {
    for (mut client, mut pos, mut loc, mut game_mode, username, props, uuid) in &mut clients {
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        loc.0 = instances.single();
        *game_mode = GameMode::Creative;

        client.send_message(
            "Please open your player list (tab key)."
                .italic()
                .color(Color::WHITE),
        );

        let entry = PlayerListEntry::new()
            .with_username(&username.0)
            .with_properties(props.0.clone()) // For the player's skin and cape.
            .with_game_mode(*game_mode)
            .with_ping(0) // Use negative values to indicate missing.
            .with_display_name(Some("ඞ".color(Color::new(255, 87, 66))));

        player_list.insert(uuid.0, entry);
    }
}

fn update_player_list(mut player_list: ResMut<PlayerList>, server: Res<Server>) {
    let tick = server.current_tick();

    player_list.set_header("Current tick: ".into_text() + tick);
    player_list
        .set_footer("Current tick but in purple: ".into_text() + tick.color(Color::LIGHT_PURPLE));

    if tick % 5 == 0 {
        let mut rng = rand::thread_rng();
        let color = Color::new(rng.gen(), rng.gen(), rng.gen());

        let entry = player_list.get_mut(PLAYER_UUID_1).unwrap();
        let new_display_name = entry.display_name().unwrap().clone().color(color);
        entry.set_display_name(Some(new_display_name));
    }

    if tick % 20 == 0 {
        match player_list.entry(PLAYER_UUID_2) {
            Entry::Occupied(oe) => {
                oe.remove();
            }
            Entry::Vacant(ve) => {
                let entry = PlayerListEntry::new()
                    .with_display_name(Some("Hello!"))
                    .with_ping(300);

                ve.insert(entry);
            }
        }
    }
}

fn remove_disconnected_clients_from_player_list(
    mut clients: RemovedComponents<Client>,
    mut player_list: ResMut<PlayerList>,
    uuids: Query<&UniqueId>,
) {
    for client in clients.iter() {
        if let Ok(UniqueId(uuid)) = uuids.get(client) {
            player_list.remove(*uuid);
        }
    }
}
