module Helpers.UEFIAssert;

import Helpers.OutputString : OutputString;
import Format : Format;
import UEFI.Base : Status;
import Conversion : StrToStr16;

pragma(inline, true) noreturn UEFIAssert(T)(T expr_, const(wchar)[] msg, string func = __PRETTY_FUNCTION__,
        string file = __FILE__, size_t line = __LINE__)
if (is(T == bool) || is(T == Status)) {
    bool expr;

    static if (is(T == bool))
        expr = expr_;
    else static if (is(T == Status))
        expr = expr_ == Status.Success;

    if (!expr) {
        wchar[512] filename, function_;
        Format("Assertion failed at {s}:{i}, {s}: {s}\n\r"w, &OutputString, file.StrToStr16(filename),
                line, func.StrToStr16(function_), msg);
        static if (is(T == Status))
            Format("Context: UEFI function called returned {x}\n\r"w, &OutputString, expr_);
        while (1) {
        }
    }
}