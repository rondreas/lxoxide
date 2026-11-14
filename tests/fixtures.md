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

## falling_cube.lxo

Creating a 5x5 plane, with a rigid body simulated cube falling on it. This should create a BAKE chunk with the simulation. Fixture created with Modo version 16.1v8.

```lxm
tool.set prim.cube on
tool.attr prim.cube sizeX 5.0
tool.attr prim.cube sizeY 0.0
tool.attr prim.cube sizeZ 5.0
tool.apply
tool.set prim.cube off

dynamics.makeDynamic type:4

script.implicit {Unit Cube Item}

transform.channel pos.Y 6.0
transform.channel rot.X 7.0
transform.channel rot.Y 36.0
transform.channel rot.Z 19.0

dynamics.makeDynamic 0

simulate start:0.0 end:2.5 preroll:0.0

scene.saveAs "path/to/fixtures/falling_cube.lxo" $LXOB false
```
