# eyebot-rs
TODO: write readme

# options

## features
Enable/disable features.

### eye
* Enables all eye-related features
* type: `bool`
* default: `true`

### custom_commands
* Enables chat-defined commands. 
* *Disabled if `features.eye` is `false`.*
* type: `bool`
* default: `true`

### counters
* Enables chat-defined counters. 
* *Disabled if `features.eye` is `false`.*
* type: `bool`
* default: `true`
  
### listeners
* Enables chat-defined listeners. 
* *Disabled if `features.eye` is `false`.*
* type: `bool`
* default: `true`

## exec
Details about how the console side of the program functions.

### debug
* Enables debug messages.
* type: `bool`
* default: `false`

## bot
Details about how the bot functions.

### duplicate_message_depth
* How many messages the bot can store in its sent message history to check in
  order to not send a duplicate message.
* *A value of `0` prevents the duplicate message check.*
* *Replies will not be checked.*
* type: `positive integer`
* default: `0`
