# LXO File Format Specification

**Version 4.2**  
Luxology, LLC

## Appendix I: LXO File Format Extensions

The LXO format is modo's native file format. At its core, it is an extended Lightwave LWO2 format, which itself is an IFF chunk format. This appendix focuses on the differences between LWO2 and LXO, and assumes familiarity with LWO2. Please refer to NewTek's documentation regarding LWO2 for basic information on reading basic IFF data and common chunks.

> **Note**: This is preliminary documentation and is subject to change. Be aware that there are likely to be large-scale changes to this format in the future as modo gets more capabilities, and entirely new file formats may be created that supplant this.

[IFFAnalizer](http://www.lightwavers.com/files/IFFAnalizer.zip) is an invaluable tool for analyzing IFF files, and could prove helpful in deconstructing existing LXO files as well as validating LXO files you create yourself. This is a Windows application, but it can also be run through CrossOver, Parallels, VMWare or one of the other virtual machines or WINE platforms on Intel-based OS X systems.

---

## Table of Contents

- [1. Conventions](#1-conventions)
  - [1.1 Datatype Shorthand](#11-datatype-shorthand)
- [2. LXO and LXP](#2-lxo-and-lxp)
- [3. LXO Chunk Hierarchy](#3-lxo-chunk-hierarchy)
  - [3.1 The Importance of Chunk Ordering](#31-the-importance-of-chunk-ordering)
  - [3.2 Hierarchy Chart](#32-hierarchy-chart)
- [4. File Headers](#4-file-headers)
  - [4.1 LXOB Header](#41-lxo-header)
  - [4.2 LXPR Header](#42-lxp-header)
- [5. Chunk Headers](#5-chunk-headers)
  - [5.1 Chunk Headers](#51-chunk-headers)
  - [5.2 Sub-Chunk Headers](#52-sub-chunk-headers)
- [6. Core Chunks](#6-core-chunks)
  - [6.1 VRSN - Version Chunk](#61-vrsn---version-chunk)
  - [6.2 CHNM - Channel Names Chunk](#62-chnm---channel-names-chunk)
- [7. Item Chunk](#7-item-chunk)
  - [7.1 ITEM Fixed Fields](#71-item-fixed-fields)
  - [7.2 XREF - External Reference Sub-Chunk](#72-xref---external-reference-sub-chunk)
  - [7.3 LAYR - Layer Sub-Chunk](#73-layr---layer-sub-chunk)
  - [7.4 UNIQ - Unique Identifier Sub-Chunk](#74-uniq---unique-identifier-sub-chunk)
  - [7.5 PAKG - Package Sub-Chunk](#75-pakg---package-sub-chunk)
  - [7.6 Variable Sub-Chunks](#76-variable-sub-chunks)
    - [7.6.1 LINK - Item Link](#761-link---item-link)
    - [7.6.2 CLNK - Channel Link](#762-clnk---channel-link)
    - [7.6.3 UCHN - User Channel Definition](#763-uchn---user-channel-definition)
    - [7.6.4 CHNL - Scalar Channel Value](#764-chnl---scalar-channel-value)
    - [7.6.5 CHNV - Vector Channel Value](#765-chnv---vector-channel-value)
    - [7.6.6 CHNS - String Channel Value](#766-chns---string-channel-value)
    - [7.6.7 GRAD - Gradient Channel Value](#767-grad---gradient-channel-value)
    - [7.6.8 CHAN - Channel Value Sub-Chunk](#768-chan---channel-value-sub-chunk)
    - [7.6.9 ITAG - Item Tag](#769-itag---item-tag)
- [8. Layer Chunk](#8-layer-chunk)
- [9. Envelope Chunk](#9-envelope-chunk)
  - [9.1 TANI - Incoming Tangent Sub-Chunk](#91-tani---incoming-tangent-sub-chunk)
  - [9.2 TANO - Outgoing Tangent Sub-Chunk](#92-tano---outgoing-tangent-sub-chunk)
  - [9.3 KEY - Key Value Sub-Chunk](#93-key---key-value-sub-chunk)
  - [9.4 FLAG - Flags Sub-Sub-Chunk](#94-flag---flags-sub-sub-chunk)
  - [9.5 PRE and POST - Pre and Post Behavior](#95-pre-and-post---pre-and-post-behavior)
  - [9.6 Weight and Slope Types](#96-weight-and-slope-types)
- [10. Action Chunk](#10-action-chunk)
  - [10.1 ACTN Fixed Fields](#101-actn-fixed-fields)
  - [10.2 PRNT - Parent Sub-Chunk](#102-prnt---parent-sub-chunk)
  - [10.3 ITEM - Action Item Sub-Chunk](#103-item---action-item-sub-chunk)
  - [10.4 CHAN - Action Channel Sub-Chunk](#104-chan---action-channel-sub-chunk)
  - [10.5 CHNN - Named Channel Sub-Chunk](#105-chnn---named-channel-sub-chunk)
  - [10.6 GRAD - Action Gradient Sub-Chunk](#106-grad---action-gradient-sub-chunk)
  - [10.7 CHNS - Action String Sub-Chunk](#107-chns---action-string-sub-chunk)
- [11. Geometry Chunks](#11-geometry-chunks)
  - [11.1 PNTS, POLS, and PTAG](#111-pnts-pols-and-ptag)
  - [11.2 Polygon Types](#112-polygon-types)
  - [11.3 VMAP - Vertex Map Chunk](#113-vmap---vertex-map-chunk)
  - [11.4 VMAD - Discontinuous Vertex Map Chunk](#114-vmad---discontinuous-vertex-map-chunk)
  - [11.5 VMED - Vertex Map Edge Discontinuity Chunk](#115-vmed---vertex-map-edge-discontinuity-chunk)
- [12. Trisurf Chunks](#12-trisurf-chunks)
  - [12.1 3GRP - Trisurf Group Header](#121-3grp---trisurf-group-header)
  - [12.2 3SRF - Trisurf Data Header](#122-3surf---trisurf-data-header)
  - [12.3 VRTS - Vertex Position Array](#123-vrts---vertex-position-array)
  - [12.4 TRIS - Triangle Array](#124-tris---triangle-array)
  - [12.5 VVEC - Vertex Vector Array](#125-vvec---vertex-vector-array)
  - [12.6 TTGS - Tag Array](#126-ttgs---tag-array)
- [13. Preview and Metadata Chunks](#13-preview-and-metadata-chunks)
  - [13.1 PRVW - Preview Image Chunk](#131-prvw---preview-image-chunk)
  - [13.2 AUTH - Author Chunk](#132-auth---author-chunk)
  - [13.3 (c) - Copyright Chunk](#133-c---copyright-chunk)
  - [13.4 ANNO - Annotation Chunk](#134-anno---annotation-chunk)
- [14. Reference Tables](#14-reference-tables)
  - [14.1 Layer Visibility Flags](#141-layer-visibility-flags)
  - [14.2 Layer Additional Flags](#142-layer-additional-flags)
  - [14.3 Channel Vector Modes](#143-channel-vector-modes)
  - [14.4 Channel Datatypes](#144-channel-datatypes)
  - [14.5 Envelope Interpolation Flags](#145-envelope-interpolation-flags)
  - [14.6 Pre/Post Behavior Modes](#146-prepost-behavior-modes)
  - [14.7 Image Types](#147-image-types)

---

## 1. Conventions

This document uses shorthand to describe the datatypes used in the LXO. For convenience, these are the same conventions used in the LWO2 documentation from NewTek. All binary datatypes are stored in **Motorola byte order**, also known as **big endian** or **network order**, with the most significant byte first.

### 1.1 Datatype Shorthand

| Shorthand | C Datatype | Description |
|-----------|------------|-------------|
| `I1` | `char` | 1-byte integer |
| `I2` | `short` | 2-byte integer |
| `I4` | `long` | 4-byte integer |
| `U1` | `unsigned char` | 1-byte unsigned integer |
| `U2` | `unsigned short` | 2-byte unsigned integer |
| `U4` | `unsigned long` | 4-byte unsigned integer |
| `F4` | `float` | IEEE 4-byte floating point number |
| `S0` | `char *` | NUL-terminated ASCII string (if odd length, padded with NUL) |
| `VX` | `short` or `int` | Variable length index (< 0xFF00 = U2, otherwise U4 with high byte discarded) |
| `ID4` | `long` | 4-byte identifier (ASCII string via bit-shifting: `('T' << 24 \| 'E' << 16 \| 'S' << 8 \| 'T')`) |

---

## 2. LXO and LXP

- **LXO**: The primary file format used by modo. Contains scene and mesh data, render settings, animation, references to images and other LXO files, and so on.
- **LXP**: A modo preset format. Contains a subset of the LXO chunks, often containing material properties.

---

## 3. LXO Chunk Hierarchy

This provides an overview of how the chunks are laid out in the LXO file. Entries marked with `‣` are chunks or sub-chunks, while those marked with `•` are specific fields within that chunk. Some sub-chunks are optional, while some may appear zero or more times in the same part of the file. Fields with dynamic sizes are marked with `??`; more information can be found in each chunk's description.

### 3.1 The Importance of Chunk Ordering

> It is important to note that the order of the chunks matters when one chunk is referenced by another chunk. In these cases, the referenced chunk must come before the chunk that references it. For example, the LAYR chunk must come before the ITEM chunk that references it, and the PNTS, POLS and chunks that may contain circular references like LINKs do not need to obey this rule.
>
> Keep this rule in mind when referencing the hierarchy chart. Although it is ordered roughly by dependency, you need to be sure to respect the rules to avoid any compatibility problems when creating your own LXO files.

### 3.2 Hierarchy Chart

```
FORM????LXOB
├── VRSN
│   ├── U4 major
│   ├── U4 minor
│   └── S0 application
├── CHNM
│   ├── U4 count
│   └── array
│       └── S0 channel name
├── LAYR (zero or more)
│   ├── U2 index
│   ├── U2 flags
│   ├── F4[3] pivot
│   ├── S0 name
│   ├── U2 parent
│   ├── F4 subdivision level
│   ├── F4 curve angle
│   ├── F4[3] scalepivot
│   ├── U4[6] unused
│   ├── U4 ref
│   ├── U2 spline patch level
│   └── U2[3] for future expansion
├── ENVL (zero or more)
│   ├── TANI
│   │   ├── U2 slope type
│   │   ├── U2 weight type
│   │   ├── F4 slope
│   │   ├── F4 weight
│   │   └── F4 value
│   ├── TANO
│   │   ├── U4 breaks
│   │   ├── U2 slope type
│   │   ├── U2 weight type
│   │   ├── F4 slope
│   │   ├── F4 weight
│   │   └── F4 value
│   ├── KEY
│   │   ├── F4 time
│   │   └── F4 value
│   ├── FLAG (deprecated)
│   │   └── U4 flags
│   ├── PRE
│   │   └── U2 behavior
│   └── POST
│       └── U2 behavior
├── PNTS
├── PTAG
├── POLS
├── VMAP (zero or more)
│   ├── ID4 type
│   ├── U2 dimension
│   ├── S0 name
│   ├── array
│   │   ├── VX index
│   │   └── F4[??] value
├── VMAD (zero or more)
│   ├── ID4 type
│   ├── U2 dimension
│   ├── S0 name
│   ├── array
│   │   ├── VX vertex index
│   │   ├── VX polygon index
│   │   └── F4[??] value
├── VMED (zero or more)
│   ├── ID4 type
│   ├── U2 dimension
│   ├── S0 name
│   ├── array
│   │   ├── VX vertex A
│   │   ├── VX vertex B
│   │   └── F4[??] value
├── ITEM (zero or more)
│   ├── S0 type
│   ├── S0 name
│   ├── U4 reference ID
│   ├── XREF (optional)
│   │   ├── U4 index
│   │   ├── S0 filename
│   │   └── S0 item identifier
│   ├── LAYR (optional)
│   │   ├── U4 index
│   │   ├── U4 flags
│   │   └── U1[4] RGBA color
│   ├── UNIQ (optional)
│   │   └── S0 identifier
│   ├── LINK (zero or more)
│   │   ├── S0 type name
│   │   ├── I4 reference ID
│   │   └── I4 index
│   ├── CLNK (zero or more)
│   │   ├── S0 graph
│   │   ├── S0 "from" channel
│   │   ├── U4 reference ID
│   │   ├── S0 "to" channel
│   │   ├── U4 "from" index
│   │   └── U4 "to" index
│   ├── PAKG (zero or more)
│   │   ├── S0 name
│   │   ├── U4 data size
│   │   └── U1[??] data array
│   ├── UCHN (zero or more)
│   │   ├── S0 channel name
│   │   ├── S0 datatype name
│   │   ├── U4 vector mode
│   │   ├── U4 flags
│   │   ├── I4 default value (integer)
│   │   ├── F4 default value (float)
│   │   ├── U2 hint count
│   │   └── array
│   │       ├── S0 hint name
│   │       └── I4 hint value
│   ├── CHNL (zero or more)
│   │   ├── S0 channel name
│   │   ├── U2 type
│   │   └── ?? value
│   ├── CHNV (zero or more)
│   │   ├── S0 base name
│   │   ├── U2 type
│   │   ├── U2 dimension
│   │   ├── array
│   │   │   ├── S0 ext
│   │   │   └── ?? value
│   ├── CHNS (zero or more)
│   │   ├── S0 channel name
│   │   └── S0 string
│   ├── GRAD (zero or more)
│   │   ├── S0 channel name
│   │   ├── VX envelope index
│   │   └── U4 flags
│   ├── CHAN (zero or more)
│   │   ├── VX channel index
│   │   ├── U2 datatype
│   │   └── ?? value
│   ├── ITAG (zero or more)
│   │   ├── U4 type
│   │   └── S0 tag
│   └── ACTN (zero or more)
│       ├── S0 name
│       ├── S0 type
│       ├── U4 reference ID
│       ├── PRNT (optional)
│       │   └── U4 parent ID
│       ├── ITEM (zero or more)
│       │   └── U4 item ID
│       ├── CHAN (zero or more)
│       │   ├── VX name index
│       │   ├── U2 type
│       │   ├── VX envelope
│       │   └── ?? value
│       ├── CHNN (zero or more)
│       │   ├── S0 name
│       │   ├── U2 type
│       │   ├── VX envelope
│       │   └── ?? value
│       ├── GRAD (zero or more)
│       │   ├── VX name
│       │   ├── VX envelope
│       │   ├── U4 flags
│       │   └── S0 name
│       └── CHNS (zero or more)
│           ├── S0 name
│           ├── VX name index
│           └── S0 string
├── 3GRP (zero or more)
│   ├── U4 count
│   ├── U4 ref
│   └── U4 flags
├── 3SRF (zero or more)
│   ├── U4 vertex count
│   ├── U4 triangle count
│   ├── U4 vector count
│   ├── U4 tag count
│   └── U4 flags count
├── VRTS
│   └── F4[??] vertex position array
├── TRIS
│   └── U4 vertex indices
├── VVEC (zero or more)
│   ├── ID4 vector type
│   ├── U4 dimension
│   ├── S0 name
│   └── F4[??] value array
├── TTGS
│   └── array
│       ├── ID4 type
│       └── S0 value
├── PRVW
│   ├── U2 image width
│   ├── U2 image height
│   ├── U4 type
│   ├── U4 flags
│   └── array image data
├── AUTH (zero or one)
│   └── S0 author
├── (c) (zero or one)
│   └── S0 copyright
└── ANNO (zero or one)
    └── S0 annotation
```

---

## 4. File Headers

### 4.1 LXOB Header

The FORM header for LXO files is `LXOB`. The first few bytes of the file are structured as follows:

```
FORM????LXOB
```

Where `????` is a U4 (4-byte unsigned integer) representing the size of the file.

### 4.2 LXPR Header

The FORM header for LXP files is `LXPR`. It is handled identically to the LXO header:

```
FORM????LXPR
```

---

## 5. Chunk Headers

### 5.1 Chunk Headers

Each chunk starts with four bytes representing the type of the chunk, followed by four bytes representing the size of the chunk.

| Datatype | Description |
|----------|-------------|
| `U4` | Chunk type, usually four ASCII characters |
| `U4` | Length of the chunk in bytes |

### 5.2 Sub-Chunk Headers

Sub-chunks are children of other chunks. They usually contain much less data, and thus have a two-byte length instead of the normal four bytes.

| Datatype | Description |
|----------|-------------|
| `U4` | Sub-chunk type, usually four ASCII characters |
| `U2` | Length of the sub-chunk in bytes |

---

## 6. Core Chunks

### 6.1 VRSN - Version Chunk

This chunk is present at the head of the file. It contains the major and minor version numbers of the file format, and the name of the application that saved the file. If you write an application that saves an LXO, you should save your application's name here. In modo 202, the major version is 1 and the minor version is 1.

| Datatype | Description |
|----------|-------------|
| `U4` | Major version number |
| `U4` | Minor version number |
| `S0` | Name of the application that saved the file |

### 6.2 CHNM - Channel Names Chunk

Channel names are repeated throughout the file. Rather than scatter them throughout and unnecessarily increase the file size, they are consolidated into this chunk. ITEM and ACTN chunks can then index into the array of channel names to get the appropriate string. The first string in the table is usually `"unknown"`.

**Header:**

| Datatype | Description |
|----------|-------------|
| `U4` | The number of elements in the following array |

**Array:**

| Datatype | Description |
|----------|-------------|
| `S0` | The name of the channel at this index |

---

## 7. Item Chunk

The LXO file contains one ITEM chunk per item in the scene, which in turn contain various sub-chunks representing the item's attributes.

### 7.1 ITEM Fixed Fields

| Datatype | Description |
|----------|-------------|
| `S0` | Name of the item's type, such as camera or mesh |
| `S0` | User name of the item. This may be an empty string if the user has not assigned a name to it |
| `U4` | Item reference ID, unique within the file. This is used to reference the item from other items |

### 7.2 XREF - External Reference Sub-Chunk

The XREF sub-chunk identifies an external reference item, and is only present if this item is indeed a reference itself.

| Datatype | Description |
|----------|-------------|
| `U4` | Index for the sub-scene in the REFS chunk |
| `S0` | Filename containing the source scene being referenced |
| `S0` | Item identifier in the source scene |

### 7.3 LAYR - Layer Sub-Chunk

The ITEM chunk may contain either a LAYR sub-chunk or a UNIQ sub-chunk, but never both. It is also possible for neither sub-chunk to be present.

The LAYR sub-chunk contains layer-specific features for the item. This consists of a layer index, flag bits, and a wireframe/element color.

| Datatype | Description |
|----------|-------------|
| `U4` | Index of the layer in the Layer List |
| `U4` | Flags describing layer-specific properties |
| `U1[4]` | Four-element array representing the RGBA element (wireframe) color in the UI |

#### Layer Visibility Flags

The first four bits of the flags represent the item's visibility:

| Mask | Name | Description |
|------|------|-------------|
| 0 | Visible | True if the layer is visible in GL |
| 1 | Hidden | True if the layer is hidden in GL |
| 2 | Foreground | Set if this layer is the foreground layer |
| 3 | Background | Set if this layer is a background layer |

> **Note**: It is possible for both the Visible and Hidden to be set, in which case the layer's visibility is in a mixed state and the true visibility is determined by the layer's children. Also note that an item may be neither a foreground nor a background item. In that case, it is not currently selected and thus not visible in GL. This is different from the Hidden/Visible state; an item can still be the foreground or background object and also be hidden.

#### Additional Layer Flags

| Bit | Name | Description |
|-----|------|-------------|
| 4 | Bounding Box | Set if this layer is displayed as a bounding box only |
| 8 | Linear Subdiv UVs | Set to use linear interpolation of UVs on subdivision surfaces |

### 7.4 UNIQ - Unique Identifier Sub-Chunk

This sub-chunk contains a unique string identifier for the item.

| Datatype | Description |
|----------|-------------|
| `S0` | String containing a unique item identifier |

### 7.5 PAKG - Package Sub-Chunk

The PAKG sub-chunk identifies extra packages of channels associated with this item. Each package defines any extra data it wants to load or save as part of the item, which makes much of this chunk opaque to a general IFF reader.

> **Note**: This sub-chunk must come before any channel value sub-chunks. Zero or more of these may be present in an ITEM chunk.

| Datatype | Description |
|----------|-------------|
| `S0` | Package name that is used to add the package and load and save its state |
| `U4` | Package data size in bytes. Note that zero is a valid size |
| `U1[Variable]` | The package's data stored as raw bytes |

### 7.6 Variable Sub-Chunks

The variable section contains zero or more of the LINK, CLNK, UCHN, CHNL, CHNV, CHNS, GRAD, CHAN, and ITAG sub-chunks. This section must come after the optional PAKG section.

#### 7.6.1 LINK - Item Link

The LINK sub-chunk relates one item to another item. Parenting is one kind of linking. LINK sub-chunks contain a graph type name, unique ID to the target item, and the index of the link. Zero or more of these may be present in an ITEM chunk.

| Datatype | Description |
|----------|-------------|
| `S0` | The name of the graph that this link belongs to, such as parent |
| `U4` | The ID of the item in the scene |
| `U4` | The index of the link |

#### 7.6.2 CLNK - Channel Link

The CLNK sub-chunk relates a channel in one item to a channel in another item. This is commonly used to drive one channel's value from another channel's value.

| Datatype | Description |
|----------|-------------|
| `S0` | The name of the graph that this link belongs to |
| `S0` | Channel name for the "from" channel. If `"(none)"`, this is a channel-to-item link |
| `U4` | Item reference ID of the "from" item containing the "from" channel |
| `S0` | Channel name for the "to" channel |
| `U4` | "From" link index |
| `U4` | "To" link index |

#### 7.6.3 UCHN - User Channel Definition

All items have a set of standard channels that vary based on the item type. These can be further extended through the use of user channels, which allow for arbitrarily defined channels to be added to the end of the item's normal channel list. The UCHN sub-chunk defines user channels. These channels behave just like other channels, and their values are stored in the file in the same way.

> **Note**: All UCHN sub-chunks must come before any other channel chunks.

| Datatype | Description |
|----------|-------------|
| `S0` | The name of the user channel, without any vector mode suffixes |
| `S0` | The name of the channel's datatype (ExoType) |
| `U4` | Vector mode, as described below |
| `U4` | Flags; currently always 0 |
| `I4` | Default value for integer channels |
| `F4` | Default value for floating point channels |
| `U2` | Number of text hints in the hints array. May be zero if there are no hints |

**Hints Array:**

| Datatype | Description |
|----------|-------------|
| `S0` | Name of the hint, restricted by standard text hint naming rules |
| `I4` | Value of the hint |

**Vector Modes:**

Most channels are simple scalar channels, meaning they represent a single component. modo also supports vector mode channels. These are defined just like normal channels, but are actually created as two or more related channels with a standard suffix automatically applied to the base channel name. For example, a color channel named `"mycolor"` defined as an RGB vector implicitly creates three scalar channels identified as `mycolor.r`, `mycolor.g` and `mycolor.b`. Each of the component channels are accessed just like normal scalar channels.

| Value | Mode | Description |
|-------|------|-------------|
| 0 | Scalar | The channel name has no suffixes applied, and only a single channel is created |
| 1 | XY | Two channels suffixed with `.x` and `.y` representing a two dimensional position, rotation or vector |
| 2 | XYZ | Three channels suffixed with `.x`, `.y` and `.z` representing a three dimensional position, rotation or vector |
| 3 | RGB | Three channels suffixed with `.r`, `.g` and `.b` representing a color |
| 4 | RGBA | Four channels suffixed with `.r`, `.g`, `.b` and `.a` representing a color plus an alpha |

#### 7.6.4 CHNL - Scalar Channel Value

The CHNL sub-chunk contains the name, type and value of an individual channel. These are commonly found in preset files and older LXO files without CHNM tables, while newer LXOs contain only CHAN chunks and matching CHNM tables.

| Datatype | Description |
|----------|-------------|
| `S0` | The name of the channel |
| `U2` | The datatype of the channel, which can be one of a series of flags |
| `[Variable]` | The value of the channel. The datatype is dependent on the previous field |

**Channel Datatypes:**

| Mask | Shorthand Type | Description |
|------|----------------|-------------|
| 1 | I4 | Signed integer |
| 2 | F4 | Floating point number |
| 3 | S0 | NUL-terminated string. This is used for raw strings and for text hints discrete choices |
| 4 | U2, `[data]` | Custom data. The U2 is the length of the data in bytes, followed by that many bytes of data. The exact format of the data is dependent on the datatype |

#### 7.6.5 CHNV - Vector Channel Value

The CHNV sub-chunk represents the values of a channel vector. This is a combination of three channel values, usually RGB or XYZ. The chunk has the base name, datatype, array dimensions, and an array of values. Zero or more of these may be present in an ITEM chunk.

These are commonly found in preset files and older LXO files without CHNM tables, while newer LXOs contain only CHAN chunks and matching CHNM tables.

| Datatype | Description |
|----------|-------------|
| `S0` | The base name of the channel. This is the channel name minus the vector component |
| `U2` | The datatype of the channel, which can be one of a series of flags, as described in CHNL above |
| `U2` | The number of elements in the vector. 3 for XYZ or RGB, four for RGBA, etc. |

**Vector Element Array:**

Following the above is an array of name/value pairs for the elements of the vector. The number of pairs is equal to the last U2 in the above description.

| Datatype | Description |
|----------|-------------|
| `S0` | The name of the vector component. This is often something like X, Y, Z, R, G, B, A, etc. Appending this to the base name will yield the full channel name |
| `[Variable]` | The actual value using the datatype defined above |

#### 7.6.6 CHNS - String Channel Value

The CHNS sub-chunk represents a string channel containing the channel name and the string value. Zero or more of these may be present in an ITEM chunk.

These are commonly found in preset files and older LXO files without CHNM tables, while newer LXOs contain only CHAN chunks and matching CHNM tables.

| Datatype | Description |
|----------|-------------|
| `S0` | Channel name |
| `S0` | String value assigned to the channel |

#### 7.6.7 GRAD - Gradient Channel Value

The GRAD sub-chunk contains the values of a gradient channel, which consists of the channel name and the index of an envelope in the ENVL chunk. Zero or more of these may be present in an ITEM chunk.

| Datatype | Description |
|----------|-------------|
| `S0` | Channel name |
| `VX` | Index of the envelope in the ENVL chunk |
| `U4` | Envelope interpolation flags |

**Envelope Interpolation Flags:**

Envelope interpolation flags determine how the path between keyframes is resolved.

| Flag | Description |
|------|-------------|
| 0 | Curve |
| 1 | Linear |
| 2 | Stepped |

#### 7.6.8 CHAN - Channel Value Sub-Chunk

The CHAN sub-chunk is a newer, more generalized mechanism used to represent a channel value. The channel name is looked up by index in the CHNM chunk. The value's type is determined by the U2 type field. Zero or more of these may be present in an ITEM chunk.

| Datatype | Description |
|----------|-------------|
| `VX` | Channel index in the CHNM chunk's array of channel names |
| `U2` | Datatype of the value. See below |
| `[Variable]` | Value of the channel |

**Channel Datatypes:**

| Type | Description |
|------|-------------|
| 1 | Integer |
| 2 | Float |
| 3 | String representing an integer text hint |
| 17 | Integer with an envelope |
| 18 | Float with an envelope |
| 19 | String representing an integer text hint with an envelope |

#### 7.6.9 ITAG - Item Tag

Items can be tagged with arbitrary strings, which are stored in ITAG sub-chunks. Zero or more of these may be present in an ITEM chunk.

| Datatype | Description |
|----------|-------------|
| `ID4` | Tag type, such as `CMNT`, `DESC` or `CUE` |
| `S0` | Tag value. This may be an empty string for some tags |

---

## 8. Layer Chunk

The LAYR chunk is used with mesh items that have a corresponding LAYR sub-chunk in their ITEM chunk. This is a combination of LWO2 data and newer LXO data.

| Datatype | Description |
|----------|-------------|
| `U2` | Legacy index for LWO2 compatibility |
| `U2` | Flags; see below |
| `F4[3]` | Rotation pivot point location, which defines the center of rotation |
| `S0` | Layer name. May be empty if the layer has not been named |
| `U2` | Legacy parent index for LWO2 compatibility. Use the LINK chunk described previously instead |
| `F4` | Refinement level used when freezing subdivision meshes into polygons for rendering. The display refinement level is a per-system user setting and is not stored in the LXO |
| `F4` | Refinement level used when freezing curves, represented as the maximum angle between adjacent linear segments |
| `F4[3]` | Scale pivot point location, which defines the center of scaling |
| `U4[6]` | Currently unused |
| `U4` | Item reference for the mesh layer |
| `U2` | Refinement level used when freezing spline patch surfaces |
| `U2[3]` | Refinement level; for future expansion |

---

## 9. Envelope Chunk

The ENVL chunk describes an envelope applied to an item. In modo, envelopes define the keys of gradients and for normal keyframed animation. Note that this is not the same as the LWO2 envelope chunk. The envelope contains three sub-chunks representing the spline, TANI, TANO and KEY, as well as the behavior chunks PRE and POST.

The spline type used in modo is a variation on the bezier spline. The specific implementation is not currently documented, but it should be close enough to standard bezier curves for you to use that at the moment.

### 9.1 TANI - Incoming Tangent Sub-Chunk

The TANI chunk contains information about the incoming tangent of the spline.

| Datatype | Description |
|----------|-------------|
| `U2` | Slope type |
| `U2` | Weight type |
| `F4` | Weight |
| `F4` | Slope |
| `F4` | Value |

### 9.2 TANO - Outgoing Tangent Sub-Chunk

TANO similarly contains the outgoing tangent. This is used only for broken keys.

| Datatype | Description |
|----------|-------------|
| `U4` | Breaks; describes which values are discontinuous |
| `U2` | Slope type |
| `U2` | Weight type |
| `F4` | Weight |
| `F4` | Slope |
| `F4` | Value |

**Break Types:**

The break type can be any combination of the value, slope or weight flags.

| Value | Description |
|-------|-------------|
| 0 | Value |
| 1 | Slope |
| 2 | Weight |

### 9.3 KEY - Key Value Sub-Chunk

This is the key value itself, which includes a key and an input value for the gradient. It also contains the FLAG sub-sub-chunk.

| Datatype | Description |
|----------|-------------|
| `F4` | Input value for the gradient |
| `F4` | Value for the key |

### 9.4 FLAG - Flags Sub-Sub-Chunk

The flags sub-sub-chunk contains client-specific flags for the keyframe. These are deprecated, and are not used in any form in any version of modo. Any FLAG chunk found can simply be ignored.

| Datatype | Description |
|----------|-------------|
| `U4` | Flags |

### 9.5 PRE and POST - Pre and Post Behavior

The pre and post behaviors define how the envelope is interpreted before and after the first keyframe.

| Datatype | Description |
|----------|-------------|
| `U2` | Pre or post behavior mode |

**Pre/Post Behavior Modes:**

| Mask | Name | Description |
|------|------|-------------|
| 0 | Reset | The default value, often zero |
| 1 | Constant | Hold the value of the nearest keyframe |
| 2 | Repeat | Repeat the envelope from the first to last keyframe |
| 3 | Oscillate | Similar to repeat, but runs the envelope forward and backward alternately |
| 4 | Offset Repeat | Similar to repeat, but the values are offset each cycle by the difference between the first and last keyframes |
| 5 | Linear | Linear interpolation derived from the slope of the nearest keyframe |
| 6 | None | Indicates that the envelope does not exist before or after the keyframe range, and the parent action's values should be used |

### 9.6 Weight and Slope Types

**Slope Types:**

In TANI and TANO, the slope type can be manual, auto, linear in or out, or flat.

| Value | Description |
|-------|-------------|
| 0 | Manual |
| 1 | Automatic |
| 2 | Linear In |
| 3 | Linear Out |
| 4 | Flat |

**Weight Types:**

The weight type can be either manual or automatic.

| Value | Description |
|-------|-------------|
| 0 | Manual |
| 1 | Automatic |

---

## 10. Action Chunk

The ACTN chunk contains information about actions. An action is a collection of channel values. There may be zero or more action chunks in the file.

### 10.1 ACTN Fixed Fields

| Datatype | Description |
|----------|-------------|
| `S0` | Name |
| `S0` | Type of action. Common types include `edit`, `setup` and `anim` |
| `U4` | Reference identifier |
| `U4` | Flags, for future use. Currently must be 0 |

### 10.2 PRNT - Parent Sub-Chunk

Actions can be layered in a parenting hierarchy. The optional PRNT sub-chunk identifies its parent layer, if any.

| Datatype | Description |
|----------|-------------|
| `U4` | Parent action's reference identifier |

### 10.3 ITEM - Action Item Sub-Chunk

Each action contains a zero or more ITEM sub-chunks, each representing an item that has channels in this action. Immediately following each of these sub-chunks may be zero or more CHAN, CHNN, GRAD or CHNS sub-chunks, each representing a channel of the previously listed ITEM sub-chunk. Note that all of these are sub-chunks of the ACTN chunk, not of the ITEM sub-chunk.

The ITEM sub-chunk itself contains the item's reference identifier.

| Datatype | Description |
|----------|-------------|
| `U4` | Item reference identifier |

### 10.4 CHAN - Action Channel Sub-Chunk

The CHAN sub-chunk contains information about a single channel's values for the preceding ITEM sub-chunk.

| Datatype | Description |
|----------|-------------|
| `VX` | Index of the channel's name in the CHNM chunk's array |
| `U2` | Datatype of the channel. See the ITEM chunk's CHAN sub-chunk |
| `VX` | Index of the envelope in the ENVL chunk's array, if applicable |
| `[Variable]` | Value of the channel. The datatype is determined by the type field |

### 10.5 CHNN - Named Channel Sub-Chunk

The CHNN sub-chunk contains information about a single channel's values for the preceding ITEM sub-chunk. This is identical to the CHAN sub-chunk, but the channel is explicitly named instead of using a lookup into the CHNM chunk's array.

| Datatype | Description |
|----------|-------------|
| `S0` | Name of the channel |
| `U2` | Type of the channel, as described in the CHAN sub-chunk above |
| `VX` | Index of the envelope in the ENVL chunk's array, if applicable |
| `[Variable]` | Value of the channel. The datatype is determined by the type field |

### 10.6 GRAD - Action Gradient Sub-Chunk

The GRAD sub-chunk contains information about a single gradient channel's values for the preceding ITEM sub-chunk.

| Datatype | Description |
|----------|-------------|
| `VX` | Index of the channel's name in the CHNM chunk's array |
| `VX` | Index of the envelope in the ENVL chunk's array, if applicable |
| `U4` | Flags |
| `S0` | Optional channel name. Used if the name index above is 0 |

### 10.7 CHNS - Action String Sub-Chunk

The CHNS sub-chunk contains information about a single string channel's values for the preceding ITEM sub-chunk.

| Datatype | Description |
|----------|-------------|
| `S0` | Name of the channel. If empty, use the following field |
| `VX` | Index of the channel in the CHNM chunk's array, if applicable |
| `S0` | The channel's value |

---

## 11. Geometry Chunks

### 11.1 PNTS, POLS, and PTAG

The chunks for vertices, polygons and polygon tags are the same as those in the LWO2 format. Please refer to the LWO2 file format specification from NewTek for more information.

### 11.2 Polygon Types

The LXO format has a few additional polygon types. Below are all of the types currently supported by modo 601.

| Datatype | Description |
|----------|-------------|
| `FACE` | Polygon |
| `CURV` | Curve |
| `BEZR` | Bezier curve |
| `SUBD` | Subdivision surface |
| `SPCH` | Spline patch, as generated by the patching tools |
| `TEXT` | Text, as generated by the Text tool |

### 11.3 VMAP - Vertex Map Chunk

The VMAP chunk holds information about vertex maps, including the type, dimensions, name and an array of vertex values.

| Datatype | Description |
|----------|-------------|
| `ID4` | Type of vertex map, such as `COLR` or `MORF` |
| `U2` | Dimensions of the vertex map |
| `S0` | Name of the vertex map, as assigned by the user |

**Vertex/Value Array:**

The remainder of the chunk consists of an array of vertex/value pairs.

| Datatype | Description |
|----------|-------------|
| `VX` | Index of the vertex in the PNTS chunk |
| `F4[n]` | Array of values associated with the vertex. n is equal to the dimensions above |

### 11.4 VMAD - Discontinuous Vertex Map Chunk

The VMAD chunk holds information about discontinuous vertex maps, which are often used for UVs. This includes the type, dimensions, name and an array of vertex values.

| Datatype | Description |
|----------|-------------|
| `ID4` | Type of vertex map, such as `COLR` or `MORF` |
| `U2` | Dimensions of the vertex map |
| `S0` | Name of the vertex map, as assigned by the user |

**Vertex/Polygon/Value Array:**

The remainder of the chunk consists of an array of values containing the vertex index, polygon index and value.

| Datatype | Description |
|----------|-------------|
| `VX` | Index of the vertex in the PNTS chunk |
| `VX` | Index of the polygon sharing this vertex in the POLS chunk |
| `F4[n]` | Array of values associated with the vertex. n is equal to the dimensions above |

### 11.5 VMED - Vertex Map Edge Discontinuity Chunk

The VMED chunk provides Vertex Map Edge Discontinuity maps. These are formatted much like the VMAP and VMAD chunks.

| Datatype | Description |
|----------|-------------|
| `ID4` | Type of vertex map, such as `SUBD` |
| `U2` | Dimensions of the vertex map |
| `S0` | Name of the vertex map, as assigned by the user |

**Edge/Value Array:**

The remainder of the chunk consists of an array of values containing the vertex index, polygon index and value.

| Datatype | Description |
|----------|-------------|
| `VX` | The "A" vertex defining the edge |
| `VX` | The "B" vertex defining the edge |
| `F4[n]` | Array of values associated with the edge. n is equal to the dimensions above |

---

## 12. Trisurf Chunks

Trisurfs are a simplified geometry representation that removes extra features from a model to allow more geometry to be loaded into memory at once. These are represented by their own series of chunks.

### 12.1 3GRP - Trisurf Group Header

The 3GRP chunk precedes any other trisurf chunks, and is a header that defines a group of 3SRF chunks. Multiple 3GRP chunks may be in the file, followed by their associated 3SRF, VRTS, VVEC and TTGS chunks.

| Datatype | Description |
|----------|-------------|
| `U4` | Number of trisurfs in the group. Should be equal to the number of 3SRF chunks following this chunk |
| `U4` | Item reference index that this group is associated with |
| `U4` | Flags for future expansion |

### 12.2 3SRF - Trisurf Data Header

The 3SRF chunk is a header that identifies a collection of geometry within a trisurf group. It is followed by its associated VRTS, VVEC and TTGS chunks.

| Datatype | Description |
|----------|-------------|
| `U4` | Number of vertices in the VRTS chunk |
| `U4` | Number of triangles in the TRIS chunk |
| `U4` | Number of VVEC chunks |
| `U4` | Number of tags in a TAGS chunk |
| `U4` | Flags for future expansion |

### 12.3 VRTS - Vertex Position Array

The VRTS chunk contains an array of vertex positions for the preceding 3SRF chunk. Each vertex is represented by three floats representing the X, Y and Z coordinate of the vertex.

| Datatype | Description |
|----------|-------------|
| `F4[n]` | Array representing the vertex positions as sets of three F4 values per vertex. n is equal to the vertex count in the preceding 3SRF chunk |

### 12.4 TRIS - Triangle Array

The TRIS chunk links the vertices from the VRTS chunk into a series of triangles. They are represented as an array of three integer vertex indices per triangle.

| Datatype | Description |
|----------|-------------|
| `U4[n]` | Array representing the triangles as sets of three U4 vertex indices per triangle. n is equal to the triangle count in the preceding 3SRF chunk |

### 12.5 VVEC - Vertex Vector Array

The VVEC chunk defines a vertex vector (aka a vertex map). There may be multiple VVEC chunks for a single trisurf, thus allowing multiple vertex vectors to be defined.

| Datatype | Description |
|----------|-------------|
| `ID4` | Type of vector, such as `COLR` or `MORF` |
| `U4` | Dimensions (number of components in the map) |
| `S0` | Name of the vector |
| `F4[n]` | Array of floats representing the vector. n is equal to the number of dimensions above |

### 12.6 TTGS - Tag Array

The TTGS chunk defines one or more tags for a given trisurf as an array.

| Datatype | Description |
|----------|-------------|
| `array[n]` | Array of ID4 and S0 pairs, as described below. n is equal to the tag count in the preceding 3SRF chunk |

Each tag is identified by an arbitrary U4 type identifier and a string containing the tag's value.

| Datatype | Description |
|----------|-------------|
| `ID4` | Arbitrary tag type |
| `S0` | Value of the tag |

---

## 13. Preview and Metadata Chunks

### 13.1 PRVW - Preview Image Chunk

The optional PRVW chunk provides a preview or thumbnail image for the file. The array of bytes is a PNG-compressed image, and can be loaded with the standard PNG file stream loader available in the libpng library.

| Datatype | Description |
|----------|-------------|
| `U2` | Image width |
| `U2` | Image height |
| `U4` | Image type, as a combination of flags described below |
| `U4` | Flags defining the array's contents. Currently 0 means uncompressed, and 1 means PNG compression |
| `I1[n]` | Array of bytes representing the image data, stored as a PNG image without any header or other decoration |

**Image Types:**

The image type can be one of the values in this table. This table is derived from a series of flags, where `0x00` is "8 Bit" and `0x08` is "Floating Point", and where `0x01` is Greyscale, `0x03` is RGB, and `0x04` is RGBA. However, only the following combinations are actually used.

| Value | Description |
|-------|-------------|
| 1 | 8 Bit Greyscale |
| 9 | 32 Bit Floating Point Greyscale |
| 3 | 8 Bit RGB |
| 12 | 32 Bit Floating Point RGB |
| 4 | 8 Bit RGBA |
| 13 | 32 Bit Floating Point RGBA |

### 13.2 AUTH - Author Chunk

The AUTH chunk contains the name of the author of the file. This could be the person's true name, a company name, their machine username, etc. While present, this data is often not preserved through load and save.

| Datatype | Description |
|----------|-------------|
| `S0` | Author's name |

### 13.3 (c) - Copyright Chunk

This chunk contains copyright information for the file, which is the date and copyright holder minus the © symbol itself. Note that the chunk name is always four characters, here ending in a space. While present, this data is often not preserved through load and save.

| Datatype | Description |
|----------|-------------|
| `S0` | Date and copyright holder, minus the © symbol |

### 13.4 ANNO - Annotation Chunk

This chunk contains textual file annotations as a string.

| Datatype | Description |
|----------|-------------|
| `S0` | Annotation string |

---

## 14. Reference Tables

### 14.1 Layer Visibility Flags

| Mask | Name | Description |
|------|------|-------------|
| 0 | Visible | True if the layer is visible in GL |
| 1 | Hidden | True if the layer is hidden in GL |
| 2 | Foreground | Set if this layer is the foreground layer |
| 3 | Background | Set if this layer is a background layer |

### 14.2 Layer Additional Flags

| Bit | Name | Description |
|-----|------|-------------|
| 4 | Bounding Box | Set if this layer is displayed as a bounding box only |
| 8 | Linear Subdiv UVs | Set to use linear interpolation of UVs on subdivision surfaces |

### 14.3 Channel Vector Modes

| Value | Mode | Description |
|-------|------|-------------|
| 0 | Scalar | The channel name has no suffixes applied, and only a single channel is created |
| 1 | XY | Two channels suffixed with `.x` and `.y` representing a two dimensional position, rotation or vector |
| 2 | XYZ | Three channels suffixed with `.x`, `.y` and `.z` representing a three dimensional position, rotation or vector |
| 3 | RGB | Three channels suffixed with `.r`, `.g` and `.b` representing a color |
| 4 | RGBA | Four channels suffixed with `.r`, `.g`, `.b` and `.a` representing a color plus an alpha |

### 14.4 Channel Datatypes

| Mask | Shorthand Type | Description |
|------|----------------|-------------|
| 1 | I4 | Signed integer |
| 2 | F4 | Floating point number |
| 3 | S0 | NUL-terminated string. This is used for raw strings and for text hints discrete choices |
| 4 | U2, `[data]` | Custom data. The U2 is the length of the data in bytes, followed by that many bytes of data. The exact format of the data is dependent on the datatype |

**CHAN Type Values:**

| Type | Description |
|------|-------------|
| 1 | Integer |
| 2 | Float |
| 3 | String representing an integer text hint |
| 17 | Integer with an envelope |
| 18 | Float with an envelope |
| 19 | String representing an integer text hint with an envelope |

### 14.5 Envelope Interpolation Flags

| Flag | Description |
|------|-------------|
| 0 | Curve |
| 1 | Linear |
| 2 | Stepped |

### 14.6 Pre/Post Behavior Modes

| Mask | Name | Description |
|------|------|-------------|
| 0 | Reset | The default value, often zero |
| 1 | Constant | Hold the value of the nearest keyframe |
| 2 | Repeat | Repeat the envelope from the first to last keyframe |
| 3 | Oscillate | Similar to repeat, but runs the envelope forward and backward alternately |
| 4 | Offset Repeat | Similar to repeat, but the values are offset each cycle by the difference between the first and last keyframes |
| 5 | Linear | Linear interpolation derived from the slope of the nearest keyframe |
| 6 | None | Indicates that the envelope does not exist before or after the keyframe range, and the parent action's values should be used |

### 14.7 Image Types

| Value | Description |
|-------|-------------|
| 1 | 8 Bit Greyscale |
| 9 | 32 Bit Floating Point Greyscale |
| 3 | 8 Bit RGB |
| 12 | 32 Bit Floating Point RGB |
| 4 | 8 Bit RGBA |
| 13 | 32 Bit Floating Point RGBA |

---

*Document generated from modo Scripting and Commands.pdf (Version 4.2)*