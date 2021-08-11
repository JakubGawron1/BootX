// Copyright (c) 2021 VisualDevelopment. All rights reserved.

const uefi = @import("std").os.uefi;
const w = @import("std").unicode.utf8ToUtf16LeStringLiteral;

pub const assert = @import("assert.zig").assert;
pub const panic = @import("panic.zig").panic;
pub const con_out_writer = @import("conoutwriter.zig").con_out_writer;
pub const openESP = @import("loadfile.zig").openESP;
pub const loadFile = @import("loadfile.zig").loadFile;

pub fn findCfgTable(guid: uefi.Guid) !uefi.tables.ConfigurationTable {
    for (uefi.system_table.configuration_table[0..uefi.system_table.number_of_table_entries]) |table| {
        if (guid.eql(table.vendor_guid))
            return table;
    }

    return error.CfgTableNotFound;
}
