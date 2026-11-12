# Homebrew tap for OpenCraft

This is the [Homebrew](https://brew.sh) tap for
[OpenCraft](https://github.com/shantanugoel/OpenCraft). OpenCraft is a native
application for the VSD StreamDock N4 Pro on macOS.

```sh
brew install --cask shantanugoel/opencraft/opencraft
```

To update OpenCraft, type `brew upgrade --cask opencraft`.

OpenCraft needs a Mac with Apple silicon and macOS 12 or later. The cask
downloads the notarized disk image from the
[releases page](https://github.com/shantanugoel/OpenCraft/releases) of
OpenCraft and puts `OpenCraft.app` in the Applications folder.

Quit the vendor software VSD Craft before you start OpenCraft. The
[user guide](https://github.com/shantanugoel/OpenCraft/blob/main/docs/user/install.md)
gives the full procedure to install, to start, and to remove OpenCraft.

The file `Casks/opencraft.rb` is written by the script
`scripts/homebrew-cask.sh` of the OpenCraft repository, for each release. Do
not change the file by hand. Report a problem in the
[issues of OpenCraft](https://github.com/shantanugoel/OpenCraft/issues).
