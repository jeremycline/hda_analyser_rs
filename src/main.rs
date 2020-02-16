// hda_analyzer_rs is a tool to analyze HDA codecs, widgets, and connections
// Copyright (C) 2020 Jeremy Cline
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along
// with this program; if not, write to the Free Software Foundation, Inc.,
// 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
extern crate glib;
extern crate gtk;
extern crate hdars;

use crate::gtk::WidgetExt;
use glib::types::Type;
use gtk::prelude::*;
use std::fs;

use hdars::{get_hda_card_info, get_hda_info, hda_verb};

fn main() -> std::io::Result<()> {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        // TODO make an error
        return Ok(());
    }

    let glade_src = include_str!("hdars.glade");
    let builder = gtk::Builder::new_from_string(glade_src);
    let window: gtk::Window = builder.get_object("main").unwrap();

    // TODO Factor all this nonsense out into a some other module for building the UI
    let node_tree: gtk::TreeView = builder.get_object("node_tree").unwrap();
    let node_tree_store: gtk::TreeStore =
        gtk::TreeStore::new(&[Type::String, Type::I32, Type::I32, Type::I32, Type::Bool]);

    // Macro to make columns?

    let name_column = gtk::TreeViewColumn::new();
    let name_renderer = gtk::CellRendererText::new();
    name_column.set_title("Card Name");
    name_column.pack_start(&name_renderer, true);
    name_column.add_attribute(&name_renderer, "text", 0);
    node_tree.append_column(&name_column);

    let device_num_column = gtk::TreeViewColumn::new();
    let device_num_renderer = gtk::CellRendererText::new();
    device_num_column.set_title("Device Number");
    device_num_column.pack_start(&device_num_renderer, true);
    device_num_column.add_attribute(&device_num_renderer, "text", 1);
    node_tree.append_column(&device_num_column);

    let card_num_column = gtk::TreeViewColumn::new();
    let card_num_renderer = gtk::CellRendererText::new();
    card_num_column.set_title("Card Number");
    card_num_column.pack_start(&card_num_renderer, true);
    card_num_column.add_attribute(&card_num_renderer, "text", 2);
    node_tree.append_column(&card_num_column);

    let codec_num_column = gtk::TreeViewColumn::new();
    let codec_num_renderer = gtk::CellRendererText::new();
    codec_num_column.set_title("Codec Number");
    codec_num_column.pack_start(&codec_num_renderer, true);
    codec_num_column.add_attribute(&codec_num_renderer, "text", 3);
    node_tree.append_column(&codec_num_column);

    let italic_column = gtk::TreeViewColumn::new();
    let italic_renderer = gtk::CellRendererText::new();
    italic_column.set_title("Italic");
    italic_column.pack_start(&italic_renderer, true);
    italic_column.add_attribute(&italic_renderer, "text", 4);
    node_tree.append_column(&italic_column);

    node_tree.set_model(Some(&node_tree_store));

    println!("Opening ioctl file");
    // TODO iterate /dev/snd/ for devices automatically
    // Also figure out how to prompt user to authenticate for permissions to read files
    let card_paths = fs::read_dir("/dev/snd/")?.filter(|dentry| {
        dentry
            .as_ref()
            .unwrap()
            .file_name()
            .into_string()
            .unwrap()
            .starts_with("controlC")
    });

    for card_path in card_paths {
        let card = fs::File::open(card_path.unwrap().path())?;
        match get_hda_card_info(&card) {
            Ok(info) => {
                println!("Got card name of {}", info.card);
                let tree_iter = node_tree_store.append(None);
                node_tree_store.set(
                    &tree_iter,
                    &[0, 1, 2, 3, 4],
                    &[&info.name, &info.card, &-1, &-1, &false],
                );

                let pfilter = "hwC".to_string() + info.card.to_string().as_str();
                let device_paths = fs::read_dir("/dev/snd/")?.filter(|dentry| {
                    dentry
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .into_string()
                        .unwrap()
                        .starts_with(pfilter.as_str())
                });
                for device_path in device_paths {
                    let device = fs::OpenOptions::new()
                        .write(true)
                        .read(true)
                        .open(device_path.unwrap().path())?;
                    match get_hda_info(&device) {
                        Ok(device_info) => {
                            println!("Got card device {}", device_info.name);
                            let codec_iter = node_tree_store.append(Some(&tree_iter));
                            node_tree_store.set(
                                &codec_iter,
                                &[0, 1, 2, 3, 4],
                                &[
                                    &device_info.name,
                                    &device_info.card_number,
                                    &device_info.device_number,
                                    &-1,
                                    &false,
                                ],
                            );

                            let (_, root_node) = hda_verb::sub_node_count(&device, 0);
                            let (total_nodes, start_node) =
                                hda_verb::sub_node_count(&device, root_node);
                            println!("Total nodes: {}, start_node: {}", total_nodes, start_node);
                            for node_id in start_node..total_nodes {
                                let node_iter = node_tree_store.append(Some(&codec_iter));
                                println!("node id {}", node_id);
                                node_tree_store.set(
                                    &node_iter,
                                    &[0, 1, 2, 3, 4],
                                    &[
                                        &device_info.name,
                                        &device_info.card_number,
                                        &device_info.device_number,
                                        &node_id,
                                        &false,
                                    ],
                                );
                                // TODO for each node probe its capabilities
                            }
                        }
                        Err(e) => println!("Got {:?}", e),
                    }
                }
            }
            Err(e) => println!("Got {:?}", e),
        }
    }

    // TODO define event handler for
    //   - about button
    //   - revert button
    //   - diff button
    //   - exp(ort) button
    //   - graph button
    //
    // TODO Populate node editor tab

    // We start the gtk main loop.
    window.show_all();
    gtk::main();
    Ok(())
}
