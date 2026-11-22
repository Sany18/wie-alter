use alloc::{vec, vec::Vec};

use jvm::{Array, ClassInstanceRef, JavaValue, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::javax::microedition::lcdui::{Graphics, Image};

const CLASS: &str = "javax/microedition/lcdui/game/TiledLayer";
// Graphics.TOP | Graphics.LEFT
const TOP_LEFT: i32 = 0x10 | 0x04;

// class javax.microedition.lcdui.game.TiledLayer
pub struct TiledLayer;

impl TiledLayer {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: CLASS,
            parent_class: Some("javax/microedition/lcdui/game/Layer"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(IILjavax/microedition/lcdui/Image;II)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setStaticTileSet",
                    "(Ljavax/microedition/lcdui/Image;II)V",
                    Self::set_static_tile_set,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("createAnimatedTile", "(I)I", Self::create_animated_tile, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setAnimatedTile", "(II)V", Self::set_animated_tile, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getAnimatedTile", "(I)I", Self::get_animated_tile, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setCell", "(III)V", Self::set_cell, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getCell", "(II)I", Self::get_cell, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("fillCells", "(IIIII)V", Self::fill_cells, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getCellWidth",
                    "()I",
                    Self::get_cell_width,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new(
                    "getCellHeight",
                    "()I",
                    Self::get_cell_height,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new(
                    "getColumns",
                    "()I",
                    Self::get_columns,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new("getRows", "()I", Self::get_rows, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                JavaMethodProto::new(
                    "paint",
                    "(Ljavax/microedition/lcdui/Graphics;)V",
                    Self::paint,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("image", "Ljavax/microedition/lcdui/Image;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("cellWidth", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("cellHeight", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("staticTileCount", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("columns", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("rows", "I", FieldAccessFlags::PRIVATE),
                // tile index of each cell, row by row: 0 empty, > 0 a static tile, < 0 an animated tile
                JavaFieldProto::new("cells", "[I", FieldAccessFlags::PRIVATE),
                // the static tile of animated tile -1, -2, ...
                JavaFieldProto::new("animatedTiles", "[I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn init(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        columns: i32,
        rows: i32,
        image: ClassInstanceRef<Image>,
        cell_width: i32,
        cell_height: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::<init>({this:?}, {columns}, {rows}, {image:?}, {cell_width}, {cell_height})");

        let _: () = jvm
            .invoke_special(
                &this,
                "javax/microedition/lcdui/game/Layer",
                "<init>",
                "(II)V",
                (columns * cell_width, rows * cell_height),
            )
            .await?;
        jvm.put_field(&mut this, CLASS, "columns", "I", columns).await?;
        jvm.put_field(&mut this, CLASS, "rows", "I", rows).await?;
        let cells = jvm.instantiate_array("I", (columns * rows) as usize).await?;
        jvm.put_field(&mut this, CLASS, "cells", "[I", cells).await?;
        let animated_tiles = jvm.instantiate_array("I", 0).await?;
        jvm.put_field(&mut this, CLASS, "animatedTiles", "[I", animated_tiles).await?;

        jvm.invoke_virtual(
            &this,
            CLASS,
            "setStaticTileSet",
            "(Ljavax/microedition/lcdui/Image;II)V",
            (image, cell_width, cell_height),
        )
        .await
    }

    async fn set_static_tile_set(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        image: ClassInstanceRef<Image>,
        cell_width: i32,
        cell_height: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::setStaticTileSet({this:?}, {image:?}, {cell_width}, {cell_height})");

        let image_width: i32 = jvm
            .invoke_virtual(&image, "javax/microedition/lcdui/Image", "getWidth", "()I", ())
            .await?;
        let image_height: i32 = jvm
            .invoke_virtual(&image, "javax/microedition/lcdui/Image", "getHeight", "()I", ())
            .await?;
        let tile_count = (image_width / cell_width) * (image_height / cell_height);

        // With fewer tiles than before, the cells and the animated tiles are cleared.
        let old_tile_count: i32 = jvm.get_field(&this, CLASS, "staticTileCount", "I").await?;
        if tile_count < old_tile_count {
            let columns: i32 = jvm.get_field(&this, CLASS, "columns", "I").await?;
            let rows: i32 = jvm.get_field(&this, CLASS, "rows", "I").await?;
            let cells = jvm.instantiate_array("I", (columns * rows) as usize).await?;
            jvm.put_field(&mut this, CLASS, "cells", "[I", cells).await?;
            let animated_tiles = jvm.instantiate_array("I", 0).await?;
            jvm.put_field(&mut this, CLASS, "animatedTiles", "[I", animated_tiles).await?;
        }

        jvm.put_field(&mut this, CLASS, "image", "Ljavax/microedition/lcdui/Image;", image)
            .await?;
        jvm.put_field(&mut this, CLASS, "cellWidth", "I", cell_width).await?;
        jvm.put_field(&mut this, CLASS, "cellHeight", "I", cell_height).await?;
        jvm.put_field(&mut this, CLASS, "staticTileCount", "I", tile_count).await?;

        let columns: i32 = jvm.get_field(&this, CLASS, "columns", "I").await?;
        let rows: i32 = jvm.get_field(&this, CLASS, "rows", "I").await?;
        jvm.invoke_virtual(
            &this,
            "javax/microedition/lcdui/game/Layer",
            "setSize",
            "(II)V",
            (columns * cell_width, rows * cell_height),
        )
        .await
    }

    async fn create_animated_tile(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, static_tile_index: i32) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::createAnimatedTile({this:?}, {static_tile_index})");

        let old: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "animatedTiles", "[I").await?;
        let count = jvm.array_length(&old).await?;
        let mut tiles = jvm.load_array(&old, 0, count).await?;
        tiles.push(static_tile_index);

        let mut animated_tiles = jvm.instantiate_array("I", count + 1).await?;
        jvm.store_array(&mut animated_tiles, 0, tiles).await?;
        jvm.put_field(&mut this, CLASS, "animatedTiles", "[I", animated_tiles).await?;

        Ok(-((count + 1) as i32))
    }

    async fn set_animated_tile(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        animated_tile_index: i32,
        static_tile_index: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::setAnimatedTile({this:?}, {animated_tile_index}, {static_tile_index})");

        let mut animated_tiles: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "animatedTiles", "[I").await?;
        jvm.store_array(&mut animated_tiles, (-animated_tile_index - 1) as usize, [static_tile_index])
            .await
    }

    async fn get_animated_tile(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, animated_tile_index: i32) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::getAnimatedTile({this:?}, {animated_tile_index})");

        let animated_tiles: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "animatedTiles", "[I").await?;
        Ok(jvm.load_array(&animated_tiles, (-animated_tile_index - 1) as usize, 1).await?[0])
    }

    async fn set_cell(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, column: i32, row: i32, tile_index: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::setCell({this:?}, {column}, {row}, {tile_index})");

        jvm.invoke_virtual(&this, CLASS, "fillCells", "(IIIII)V", (column, row, 1, 1, tile_index))
            .await
    }

    async fn get_cell(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, column: i32, row: i32) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::getCell({this:?}, {column}, {row})");

        let columns: i32 = jvm.get_field(&this, CLASS, "columns", "I").await?;
        let cells: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "cells", "[I").await?;
        Ok(jvm.load_array(&cells, (row * columns + column) as usize, 1).await?[0])
    }

    #[allow(clippy::too_many_arguments)]
    async fn fill_cells(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        column: i32,
        row: i32,
        num_columns: i32,
        num_rows: i32,
        tile_index: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::fillCells({this:?}, {column}, {row}, {num_columns}, {num_rows}, {tile_index})");

        let columns: i32 = jvm.get_field(&this, CLASS, "columns", "I").await?;
        let mut cells: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "cells", "[I").await?;
        for y in row..row + num_rows {
            jvm.store_array(&mut cells, (y * columns + column) as usize, vec![tile_index; num_columns as usize])
                .await?;
        }

        Ok(())
    }

    async fn get_cell_width(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::getCellWidth({this:?})");

        jvm.get_field(&this, CLASS, "cellWidth", "I").await
    }

    async fn get_cell_height(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::getCellHeight({this:?})");

        jvm.get_field(&this, CLASS, "cellHeight", "I").await
    }

    async fn get_columns(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::getColumns({this:?})");

        jvm.get_field(&this, CLASS, "columns", "I").await
    }

    async fn get_rows(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::getRows({this:?})");

        jvm.get_field(&this, CLASS, "rows", "I").await
    }

    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, graphics: ClassInstanceRef<Graphics>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.TiledLayer::paint({this:?}, {graphics:?})");

        let layer = "javax/microedition/lcdui/game/Layer";
        let visible: bool = jvm.invoke_virtual(&this, layer, "isVisible", "()Z", ()).await?;
        if !visible {
            return Ok(());
        }
        let x: i32 = jvm.invoke_virtual(&this, layer, "getX", "()I", ()).await?;
        let y: i32 = jvm.invoke_virtual(&this, layer, "getY", "()I", ()).await?;
        let image: ClassInstanceRef<Image> = jvm.get_field(&this, CLASS, "image", "Ljavax/microedition/lcdui/Image;").await?;
        let cell_width: i32 = jvm.get_field(&this, CLASS, "cellWidth", "I").await?;
        let cell_height: i32 = jvm.get_field(&this, CLASS, "cellHeight", "I").await?;
        let columns: i32 = jvm.get_field(&this, CLASS, "columns", "I").await?;
        let rows: i32 = jvm.get_field(&this, CLASS, "rows", "I").await?;
        let cells: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "cells", "[I").await?;
        let cells: Vec<i32> = jvm.load_array(&cells, 0, (columns * rows) as usize).await?;
        let animated_tiles: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "animatedTiles", "[I").await?;
        let animated_tiles: Vec<i32> = jvm.load_array(&animated_tiles, 0, jvm.array_length(&animated_tiles).await?).await?;
        let image_width: i32 = jvm
            .invoke_virtual(&image, "javax/microedition/lcdui/Image", "getWidth", "()I", ())
            .await?;
        let tiles_per_row = image_width / cell_width;

        // only the cells in the clip
        let graphics_class = "javax/microedition/lcdui/Graphics";
        let clip_x: i32 = jvm.invoke_virtual(&graphics, graphics_class, "getClipX", "()I", ()).await?;
        let clip_y: i32 = jvm.invoke_virtual(&graphics, graphics_class, "getClipY", "()I", ()).await?;
        let clip_width: i32 = jvm.invoke_virtual(&graphics, graphics_class, "getClipWidth", "()I", ()).await?;
        let clip_height: i32 = jvm.invoke_virtual(&graphics, graphics_class, "getClipHeight", "()I", ()).await?;
        let first_column = (clip_x - x).div_euclid(cell_width).max(0);
        let last_column = (clip_x + clip_width - x - 1).div_euclid(cell_width).min(columns - 1);
        let first_row = (clip_y - y).div_euclid(cell_height).max(0);
        let last_row = (clip_y + clip_height - y - 1).div_euclid(cell_height).min(rows - 1);

        for row in first_row..=last_row {
            for column in first_column..=last_column {
                let mut tile = cells[(row * columns + column) as usize];
                if tile < 0 {
                    tile = animated_tiles[(-tile - 1) as usize];
                }
                if tile == 0 {
                    continue;
                }
                let source_x = (tile - 1) % tiles_per_row * cell_width;
                let source_y = (tile - 1) / tiles_per_row * cell_height;
                let _: () = jvm
                    .invoke_virtual(
                        &graphics,
                        graphics_class,
                        "drawRegion",
                        "(Ljavax/microedition/lcdui/Image;IIIIIIII)V",
                        [
                            image.clone().into(),
                            source_x.into(),
                            source_y.into(),
                            cell_width.into(),
                            cell_height.into(),
                            0.into(),
                            (x + column * cell_width).into(),
                            (y + row * cell_height).into(),
                            TOP_LEFT.into(),
                        ] as [JavaValue; 9],
                    )
                    .await?;
            }
        }

        Ok(())
    }
}
