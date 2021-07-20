module Application;

import UEFI.Base : Status, MemoryType, MemoryDescriptor;
import UEFI.SystemTable;
import UEFI.Protocols.ACPI;
import UEFI.Protocols.LoadedImage;
import UEFI.Protocols.SimpleFilesystem;
import UEFI.Protocols.File;
import UEFI.BootServices;
import Format : Format;
import Helpers.UEFIAssert : UEFIAssert;

__gshared SystemTable* gSystemTable = null;
__gshared BootServices* gBootServices = null;

extern (Windows) private Status FWLMain(void* imageHandle, SystemTable* systemTable)  {
    gSystemTable = systemTable;
    gBootServices = gSystemTable.bootServices;

    gSystemTable.consoleOut.EnableCursor(false);
    gSystemTable.consoleOut.ClearScreen();
    gSystemTable.consoleOut.OutputString("Welcome.\n\r"w);

    LoadedImage* image = null;
    UEFIAssert(gBootServices.HandleProtocol(imageHandle, &loadedImageGUID, cast(void**)&image), "Unable to open image info."w);
    SimpleFilesystem* fs = null;
    UEFIAssert(gBootServices.HandleProtocol(image.deviceHandle, &simpleFilesystemGUID, cast(void**)&fs), "Unable to open filesystem."w);
    File* esp = null;
    UEFIAssert(fs.OpenVolume(&esp), "Unable to open volume."w);

    if (gSystemTable.FindTable!void(ACPI.v2TableGUID))
        gSystemTable.consoleOut.OutputString("Found ACPI 2+.\n\r"w);
    else if (gSystemTable.FindTable!void(ACPI.v1TableGUID))
        gSystemTable.consoleOut.OutputString("Found ACPI 1.\n\r"w);
    else {
        gSystemTable.consoleOut.OutputString("No ACPI found, operation cannot continue."w);
        while (1) {
        }
    }

    size_t memoryMapSize, mapKey, descriptorSize;
    uint descriptorVersion;
    MemoryDescriptor* memoryMap = null;
    // Get memory map size
    gBootServices.GetMemoryMap(&memoryMapSize, memoryMap, &mapKey, &descriptorSize, &descriptorVersion);
    // Add new entry added by next allocation
    memoryMapSize += descriptorSize;
    UEFIAssert(gBootServices.AllocatePool(MemoryType.LoaderData, memoryMapSize, cast(void**)&memoryMap), "Unable to allocate memory map."w);
    // Get memory map
    UEFIAssert(gBootServices.GetMemoryMap(&memoryMapSize, memoryMap, &mapKey, &descriptorSize, &descriptorVersion), "Unable to get memory map."w);

    // Exit boot services
    UEFIAssert(gBootServices.ExitBootServices(imageHandle, mapKey), "Unable to exit boot services."w);

    // Jump to kernel code
    // TODO: Actually do the thing

    while (1) {
    }
}