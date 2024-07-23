# Subterfuge

![Preview](./res/preview.png) 

Detects changes in a Steam profile's most recently played game

🦀 written in Rust

## Why Subterfuge?

Subterfuge was born from the special characteristics of Steam's "invisible" online presence.

This mode allows you to play games online while appearing offline to others on Steam. However, this does not prevent your playtime history from being updated while you play games in secret.

Subterfuge automates the operation of evaluating Steam activity despite online status.

## How it works

Subterfuge leverages the [Steam Web API](https://steamcommunity.com/dev) to report live changes in user playtime history. User playtime is updated:

1. Every 30 minutes of a game session, and
2. On terminating a game session.

Users are supplied to Subterfuge via a list of SteamIDs. There are numerous ways to find a user's SteamID:

+ [Steam Support — "How can I find my SteamID?"](https://help.steampowered.com/en/faqs/view/2816-BE67-5B69-0FEC)
+ Third party sites
  + [steamidfinder](https://www.steamidfinder.com/) 
  + [steamid.io](https://steamid.io/)

## Configuration

Here is a template for the YAML structure that subterfuge expects.

```yaml
# ~/.config/subterfuge/config.yaml

users: # key under which all users are declared
  first_user: # arbitrary label, can be whatever you want!
    id: 76561198000000000 # steam ID
  second_user:
    id: 76561198000000001
  # ...
```
