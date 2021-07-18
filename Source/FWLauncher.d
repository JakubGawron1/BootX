module FWLauncher;

import UEFI.Base : Status, MemoryType, MemoryDescriptor;
import UEFI.SystemTable;
import UEFI.Protocols.ACPI;
import UEFI.Protocols.LoadedImage;
import UEFI.Protocols.SimpleFilesystem;
import UEFI.Protocols.File;
import UEFI.BootServices;

__gshared SystemTable* gSystemTable = null;
__gshared BootServices* gBootServices = null;

extern (Windows) Status fwMain(void* imageHandle, SystemTable* systemTable) @noreturn {
    gSystemTable = systemTable;
    gBootServices = gSystemTable.bootServices;

    gSystemTable.consoleOut.EnableCursor(false);
    gSystemTable.consoleOut.ClearScreen();
    gSystemTable.consoleOut.OutputString("Welcome.\r\n"w);

    LoadedImage* image = null;
    gBootServices.HandleProtocol(imageHandle, &loadedImageGUID, cast(void**)&image);
    SimpleFilesystem* fs = null;
    gBootServices.HandleProtocol(image.deviceHandle, &simpleFilesystemGUID, cast(void**)&fs);
    File* esp = null;
    fs.OpenVolume(&esp);

    if (gSystemTable.FindTable!void(ACPI.v2TableGUID))
        gSystemTable.consoleOut.OutputString("Found ACPI 2+.\r\n"w);
    else if (gSystemTable.FindTable!void(ACPI.v1TableGUID))
        gSystemTable.consoleOut.OutputString("Found ACPI 1.\r\n"w);
    else {
        gSystemTable.consoleOut.OutputString("No ACPI found, operation cannot continue."w);
        while (1) {}
    }

    size_t memoryMapSize, mapKey, descriptorSize;
    uint descriptorVersion;
    MemoryDescriptor* memoryMap = null;
    // Get memory map size
    gBootServices.GetMemoryMap(&memoryMapSize, memoryMap, &mapKey, &descriptorSize, &descriptorVersion);
    // Add new entry added by next allocation
    memoryMapSize += descriptorSize;
    gBootServices.AllocatePool(MemoryType.LoaderData, memoryMapSize, cast(void**)&memoryMap);
    // Get memory map
    gBootServices.GetMemoryMap(&memoryMapSize, memoryMap, &mapKey, &descriptorSize, &descriptorVersion);

    // Exit boot services
    gBootServices.ExitBootServices(imageHandle, mapKey);

    // Jump to kernel code
    // SOON

    while (1) {}
}