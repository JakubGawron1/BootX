pub const assert = @import("assert.zig").assert;
pub const panic = @import("panic.zig").panic;

const uefi = @import("std").os.uefi;

pub fn findCfgTable(guid: uefi.Guid) !uefi.tables.ConfigurationTable {
    var i: usize = 0;
    while (i < uefi.system_table.number_of_table_entries) : (i += 1) {
        const table = uefi.system_table.configuration_table[i];

        if (guid.eql(table.vendor_guid)) return table;
    }

    return error.CfgTableNotFound;
}