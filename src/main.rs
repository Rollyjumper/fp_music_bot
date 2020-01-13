extern crate clap;
extern crate ctrlc;

use std::sync::Arc;

// Import the client's bridge to the voice manager. Since voice is a standalone
// feature, it's not as ergonomic to work with as it could be. The client
// provides a clean bridged integration with voice.
use serenity::client::bridge::voice::ClientVoiceManager;

// Import the `Context` from the client and `parking_lot`'s `Mutex`.
//
// `parking_lot` offers much more efficient implementations of `std::sync`'s
// types. You can read more about it here:
//
// <https://github.com/Amanieu/parking_lot#features>
use serenity::{client::Context, prelude::EventHandler, prelude::Mutex};

use serenity::{
    client::Client,
    framework::{
        standard::{
            macros::{command, group},
            Args, CommandResult,
        },
        StandardFramework,
    },
    model::{channel::Message, gateway::Ready, misc::Mentionable},
    voice, Result as SerenityResult,
};

// This imports `typemap`'s `Key` as `TypeMapKey`.
//use serenity::prelude::*;
use serenity::prelude::TypeMapKey;

pub mod vcb_audio_source;

use clap::{App, Arg, SubCommand};
use vcb_audio_source::VCBAudioSource;

group!({
    name: "general", 
    commands: [join, leave, play],
});
//struct General;

struct InputDevice;

impl TypeMapKey for InputDevice {
    type Value = String;
}

struct VoiceManager;

impl TypeMapKey for VoiceManager {
    type Value = Arc<Mutex<ClientVoiceManager>>;
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

fn main() {
    let matches = App::new("fp-music-bot")
        .about("Discord music bot that streams any sounds from a virtual audio cable to a voice channel")
        .arg(
            Arg::with_name("token")
                .long("token")
                .short("t")
                .env("FPMUSICBOT_DISCORD_TOKEN")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("device")
                .long("input-device")
                .alias("device")
                .short("i")
                .env("FPMUSICBOT_INPUT_DEVICE")
                .takes_value(true),
        )
        .subcommand(SubCommand::with_name("list").about("Lists all compatible input devices"))
        .get_matches();

    if matches.is_present("list") {
        println!("List of input devices : ");
        let input_devices = VCBAudioSource::get_input_devices().unwrap();
        if input_devices.len() == 0 {
            println!("None !")
        } else {
            for d in input_devices {
                println!("{}", d);
            }
        }
        std::process::exit(0)
    }

    let token = match matches.value_of("token") {
        Some(token) => token,
        None => {
            println!("error: No valid Discord token provided for the bot.\nUse --help for more informatinons.");
            std::process::exit(1)
        }
    };
    let device = match matches.value_of("device") {
        Some(device) => device,
        None => {
            println!("error: No input device for the bot.\nUse --help for more informations.");
            std::process::exit(1)
        }
    };

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    // Obtain a lock to the data owned by the client, and insert the client's
    // voice manager into it. This allows the voice manager to be accessible by
    // event handlers and framework commands.
    {
        let mut data = client.data.write();
        data.insert::<VoiceManager>(Arc::clone(&client.voice_manager));
        data.insert::<InputDevice>(device.to_string());
    }

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("~"))
            .group(&GENERAL_GROUP),
    );
    let shard_manager = client.shard_manager.clone();

    ctrlc::set_handler(move || {
        shard_manager.lock().shutdown_all();
        std::process::exit(0)
    })
    .expect("Error setting Ctrl-C handler");

    let _ = client
        .start()
        .map_err(|why| println!("Client ended: {:?}", why));
}

#[command]
fn join(ctx: &mut Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache) {
        Some(guild) => guild,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Groups and DMs not supported"),
            );

            return Ok(());
        }
    };

    let guild_id = guild.read().id;

    let channel_id = guild
        .read()
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    //
    // TODO : how to check that we have permission to join ? 
    //
    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(&ctx, "Not in a voice channel"));

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let mut manager = manager_lock.lock();

    if manager.join(guild_id, connect_to).is_some() {
        check_msg(
            msg.channel_id
                .say(&ctx.http, &format!("Joined {}", connect_to.mention())),
        );
    } else {
        check_msg(msg.channel_id.say(&ctx.http, "Error joining the channel"));
    }

    Ok(())
}

#[command]
fn leave(ctx: &mut Context, msg: &Message) -> CommandResult {
    let guild_id = match ctx.cache.read().guild_channel(msg.channel_id) {
        Some(channel) => channel.read().guild_id,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Groups and DMs not supported"),
            );

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let mut manager = manager_lock.lock();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        manager.remove(guild_id);

        check_msg(msg.channel_id.say(&ctx.http, "Left voice channel"));
    } else {
        check_msg(msg.reply(&ctx, "Not in a voice channel"));
    }

    Ok(())
}

#[command]
fn play(ctx: &mut Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = match ctx.cache.read().guild_channel(msg.channel_id) {
        Some(channel) => channel.read().guild_id,
        None => {
            check_msg(msg.channel_id.say(&ctx.http, "Error finding channel info"));

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let mut manager = manager_lock.lock();

    if let Some(handler) = manager.get_mut(guild_id) {
        let data = ctx.data.read();
        let device = data.get::<InputDevice>();

        let mut vcba = VCBAudioSource::new(device.unwrap().to_string())
            .expect("Problem creating VCBAudioSource");
        match vcba.open() {
            Ok(()) => (),
            Err(why) => {
                println!("Err starting source: {:?}", why);

                check_msg(
                    msg.channel_id
                        .say(&ctx.http, "Error sourcing VCBAudioSource"),
                );

                return Ok(());
            }
        };

        handler.play(voice::pcm(vcba.is_stereo(), vcba));

        check_msg(msg.channel_id.say(&ctx.http, "Playing song"));
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in"),
        );
    }

    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}
