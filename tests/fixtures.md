# Fixtures

Guide in how to recreate the fixtures from Modo. Paths will be generic, but should be absolute.

## default.lxo

This should be achieved by just opening a Modo session, preferably in Safe Mode so we don't load any conflicting default scene or plug-ins that could change behaviours.

```lxm
scene.saveAs "path/to/fixtures/default.lxo" $LXOB false
```

## cube.lxm

Simple preset mesh item, from a Modo session we can save.

@todo - also add thumbnail here

```lxm
!!item.delete
script.implicit {Unit Cube Item}
mesh.presetSave "path/to/fixtures/cube.lxl" "This is a comment!"
```
