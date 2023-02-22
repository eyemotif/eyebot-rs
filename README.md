# eyebot-rs
TODO: write readme
- [eyebot-rs](#eyebot-rs)
- [eye](#eye)
  - [misc builtin commands](#misc-builtin-commands)
  - [custom commands](#custom-commands)
    - [custom command format](#custom-command-format)
  - [counters](#counters)
  - [listeners](#listeners)
  - [disk interactions](#disk-interactions)
- [options](#options)
  - [features](#features)
    - [eye](#eye-1)
    - [custom\_commands](#custom_commands)
    - [counters](#counters-1)
    - [listeners](#listeners-1)
  - [exec](#exec)
    - [debug](#debug)
  - [bot](#bot)
    - [duplicate\_message\_depth](#duplicate_message_depth)

# eye
Built-in functionality to make the program act more like a bot.

Note that text in `<angle brackets>` represent a required argment, and text in
`[square brackets]` represent an optional argument.

## misc builtin commands

* `!ping` *mod only*: Replies "Pong!"
* `!shutdown` *mod only*: Gracefully shuts down the program.
* `!commands`: Lists all commands that the user can execute, including [custom commands](#custom-commands).

## custom commands

Enables the usage, definition, and storage of custom commands. They are edited
using Twitch chat, with some built-in commands, all of which are *mod-only*:

* `!cmd:set <command-name> <command>`: Creates or redefines a command
  `!<command-name>`. See [custom command format](#custom-command-format) to see what to put in
  `<command>`.
* `!cmd:info <command-name>`: Outputs a command in its raw form, with its Tags
  and Variables spelled out.
* `!cmd:remove <command-name>`: Removes a custom command.

### custom command format

There are three types of data in a custom command: Text, Variables, and Tags.

* Text is just plain text. What is input will be output as-is.
* Variables are pieces of text that can change per-execution. All variables
  start with `%`:
  * `%name`: The display name of the command caller.
  * `%<number>`: The `number`-th argument of the command. If something like
    `!command a b c` is called, `a` is `%0`, `%b` is `%1`, and so on.
  * `%counter=<counter-name>`: The value of the [counter](#counters) defined by `<counter-name>`. 
* Tags are metadata that are not output, but tell the command how to execute.
  All tags start with `&`:
  *  `&REPLY`: Replies to the command caller instead of just sending a chat message.
  *  `&SUPER`: Prevents non-mods from calling the command.
  *  `&TEMP`: Prevents the command from being [saved to disk](#disk-interactions).
  *  `&C:INC=<counter-name>`: Increments the [counter](#counters) defined by `<counter-name>`.
  *  `&C:DEC=<counter-name>`: Decrements the [counter](#counters) defined by `<counter-name>`.
  *  `&C:ZERO=<counter-name>`: Sets the value of the [counter](#counters) defined by
     `<counter-name>` to zero.

Note that variable names only contain letters, numbers, `=`, `_`, and `:`. Any
other characters will be parsed as Text, so something like `@%name!` would
output correctly.

Also note that if you want to put a `%` or `&` in some Text, or separate Text
from a Variable without using a space, you must escape it with a backslash:
* `\%` -> `%`
* `\&` -> `&`
* `\\` -> `\`

## counters

Counters are just numbers associated with a name. They are edited
using Twitch chat, with some built-in commands, all of which are *mod-only*:

* `!counter:set <counter-name> <value>` Creates and/or sets a counter defined by
  `<counter-name>` to `<value>`.
* `!counter:get <counter-name>`: Outputs the value of a counter.
* `!counter:remove <counter-name>`: Removes a counter.
* `!counter:list`: Lists all counters' names.

## listeners

Listeners are kind of like more generic [custom commands](#custom-commands), where when the listener's
Pattern matches on any text, a [command](#custom-command-format) executes. They are edited
using Twitch chat, with some built-in commands, all of which are *mod-only*:

* `!listen:exact <listener-name> <pattern>/<command>`: When a chat message is
  *exactly* `<pattern>`, the listener executes.
* `!listen:has <listener-name> <pattern>/<command>`: When a chat message
  *contains* `<pattern>`, the listener executes.
* `!listen:regex <listener-name> <pattern>/<command>`: When a chat message
  *matches the regex* `<pattern>`, the listener executes.
* `!listen:info <listener-name>`: Outputs a listener and its
  [command](#custom-command-format) in its raw form, with its Pattern, Tags and Variables
  spelled out.
* `!listen:remove <listener-name>`: Removes a listener.
* `!listen:list`: Lists all listeners' names.

Note that forward slashes in a listener's Pattern must be escaped with a
backslash:
* `\/` -> `/`
* `\\` -> `\`

## disk interactions

When any [custom commands], [counters], or [listeners] are in some way created,
edited, or removed, a file on the disk is edited. Where these files are located
is controlled by the `--store` flag, or `~/.eyebot-store/` by default. 

# options

## features
Enable/disable features.

### eye
* Enables all eye-related features
* type: `bool`
* default: `true`

### custom_commands
* Enables [chat-defined commands](#custom-commands). 
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
