pub const assert = @import("assert.zig").assert;
pub const panic = @import("panic.zig").panic;

const uefi = @import("std").os.uefi;

pub fn findCfgTable(guid: uefi.Guid) !uefi.tables.ConfigurationTable {
    for (uefi.system_table.configuration_table[0..uefi.system_table.number_of_table_entries]) |table|
        if (guid.eql(table.vendor_guid)) return table;

    return error.CfgTableNotFound;
}