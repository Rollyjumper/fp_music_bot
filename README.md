# fp_music_bot
This is a Discord bot written in Rust for streaming music from a Virtual Audio Cable to Discord voice channel. This is a little project I have done for a friend who was asking a bot like this one which can be launched only with a simple command line. 

I am looking for all feedbacks you can provide. Thank you. 

The bot uses [Cpal](https://docs.rs/cpal) and [Serenity](https://docs.rs/serenity).

Be warned : I did not try to compile this for anything else than **Windows 10**.

## Compilation

Obviouly, you need to have [Rust](https://www.rust-lang.org) installed on your system. 

`cargo build --release`

The binary will be present in `target/release`.

## Prerequisites

Of course, you must install (if it is not already the case) some kind of virtual audio cable. I suggest this one : https://www.vb-audio.com/Cable. 

**Important Notes** : The input device for your virtual cable must be configured by default to use at least __48000hz__ for sample rate. It is necessary because Discord use `opus` codec for voice channels which does not support lesser than 48000hz sample rate (if I understood correctly).

In addition, you need a Discord bot token (there are numerous walkthroughs on the web, explaining how to get one). 

At last and obviouly, you need to invite your bot to a Discord server (like above there are numerous walkthroughs explaining how to do that).

## Usage

Here is the command line help : 

```
fp-music-bot
Discord music bot that streams any sounds from a virtual audio cable to a voice channel

USAGE:
    fp-music-bot.exe [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i, --input-device <device>     [env: FPMUSICBOT_INPUT_DEVICE=]
    -t, --token <token>             [env: FPMUSICBOT_DISCORD_TOKEN=]

SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    list    Lists all compatible input devices

```

When your bot is connected to your Discord server, you must connect to a voice channel and then use theses commands : 

* `~join` : the bot will join the voice channel where you are connected
* `~play` : the bot will start streaming any sound sent to your virtual audio cable
* `~leave` : the bot will leave the voice channel 

So, to play some sounds or musics, configure your player to send outputs to you virtual audio cable. 


