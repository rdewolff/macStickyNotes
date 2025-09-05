# md-sticky

<video src="https://github.com/andrewyur/md-sticky/raw/refs/heads/main/static/Demo%20Video.mp4" autoplay controls loop></video>

## About

A sticky note app inspired by the "stickies" app that comes with MacOS. I found it frustrating that it did not support md syntax, and had a ton of unecessary formatting options.

This app is not tested on windows or linux, so there may be bugs, but I don't forsee any problems getting it to work on either.

## Features

- uses a markdown text editor, compatible with github-markdown syntax (`[ ]` to make checkboxes)
- customizable colors and a large default color palate
- minimal and unobtrusive sticky note appearance
- autosave, notes persist after quitting and reopening the app
- easily move, navigate, resize, and set colors of notes with keyboard shortcuts
- automatic updates, only install once and benifit from further updates

## Installation

### Macos

I do not have an apple developer account, so the installer is not signed/notarized
download the .dmg and run `xattr -d com.apple.quarantine /path/to/dmg.dmg` to allow it to be opened

## App Specific Keyboard shortcuts

Default editor shortcuts (Cmd+X, Cmd+V, Cmd+C) are enabled

| Command                           | Action                                                                                              |
|-----------------------------------|-----------------------------------------------------------------------------------------------------|
| `Cmd + Q`                         | Quit the application                                                                                |
| `Cmd + W`                         | Close currently focused note                                                                        |
| `Cmd + N`                         | Create new note                                                                                     |
| `Cmd + /`                         | Focus next note                                                                                     |
| `Cmd + Alt + /`                   | Focus previous note                                                                                 |
| `Cmd + F`                         | Resize note to text                                                                                 |
| `Cmd + Alt + <Arrow Key>`         | Snap Note (Move window in direction until it aligns with the nearest fully overlapping window edge) |
| `Cmd + Shift + Alt + <Arrow Key>` | Partially Snap Note (Move window in direction until it aligns with the nearest window edge)         |
