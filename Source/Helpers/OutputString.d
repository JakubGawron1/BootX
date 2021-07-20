module Helpers.OutputString;

import Application : gSystemTable;

void OutputString(const(wchar)[] str) {
    gSystemTable.consoleOut.OutputString(str);
}