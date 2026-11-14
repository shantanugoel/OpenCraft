# 0039: The engine starts with the model of the last device that connected

- Status: Accepted
- Date: 2026-11-15

## Context

Issue [#156](https://github.com/shantanugoel/OpenCraft/issues/156). Before
a device connects, the engine shows a layout, a list of profiles, and device
settings. The engine used the first descriptor of the table, the N4 Pro, for
this.

With one model this is correct. With a second model it is not correct. A
user who has only the second model sees the N4 Pro in the window until the
device connects. A change that the user makes in that time goes to a profile
of the N4 Pro. When the device connects, the engine changes to the profiles
of the correct model ([decision 0018](0018-many-profiles.md)), so no data is
lost, but the window shows the wrong device for some time.

## Decision

`state.toml` in the profiles directory gets one more top key, `last_model`.
Its value is the model identifier of the last device that connected, for
example `n4-pro`.

- The engine writes the key when a device connects, if the key has a
  different value. A device of the same model as the key causes no write.
- At the start, the engine uses the descriptor of that model for the layout,
  the profiles, and the device settings, as if a device of that model were
  present. The function `runtime::start_descriptor` makes the choice.
- If the file is missing, cannot be read, has no key, or names a model that
  is not in the table, the engine uses the first descriptor of the table.
- The key is not a schema change. The other keys of the file stay as
  [decision 0035](0035-front-application-selects-the-profile.md) says. The
  engine does not use the key for anything else. A device of a different
  model that connects later gets its own layout and profiles, and the key
  then names that model.

## Results

- A user of one model sees the layout of that model at each start, with no
  device.
- A user who has two models sees the model that was last in use. If the
  other device connects, the window changes, as it did before.
- The file has no schema version. A file of an older build has no key, and
  the first descriptor is the start model. No migration is necessary.
- The first descriptor stays the start model of a new installation.
