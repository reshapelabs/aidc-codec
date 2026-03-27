#!/usr/bin/env python3
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
REF = ROOT / "gs1-reference"
OUT = pathlib.Path(__file__).resolve().parent


def extract_calls(source: str, name: str):
    needle = name + "("
    out = []
    i = 0
    while True:
        j = source.find(needle, i)
        if j < 0:
            break
        k = j + len(needle)
        depth = 1
        in_str = False
        esc = False
        while k < len(source):
            c = source[k]
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == '"':
                    in_str = False
            else:
                if c == '"':
                    in_str = True
                elif c == "(":
                    depth += 1
                elif c == ")":
                    depth -= 1
                    if depth == 0:
                        end = k + 1
                        while end < len(source) and source[end].isspace():
                            end += 1
                        if end < len(source) and source[end] == ";":
                            out.append(source[j : end + 1])
                            i = end + 1
                        else:
                            i = k + 1
                        break
            k += 1
        else:
            break
    return out


def split_args(call: str):
    inner = call[call.find("(") + 1 : call.rfind(")")]
    out = []
    cur = []
    depth = 0
    in_str = False
    esc = False
    for ch in inner:
        if in_str:
            cur.append(ch)
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
            cur.append(ch)
        elif ch == "(":
            depth += 1
            cur.append(ch)
        elif ch == ")":
            depth -= 1
            cur.append(ch)
        elif ch == "," and depth == 0:
            out.append("".join(cur).strip())
            cur = []
        else:
            cur.append(ch)
    if "".join(cur).strip():
        out.append("".join(cur).strip())
    return out


def decode_c_expr(expr: str) -> str:
    out = bytearray()
    i = 0
    b = expr.encode()
    while i < len(b):
        while i < len(b) and chr(b[i]).isspace():
            i += 1
        if i >= len(b):
            break
        if chr(b[i]) != '"':
            i += 1
            continue
        i += 1
        while i < len(b):
            c = chr(b[i])
            if c == '"':
                i += 1
                break
            if c != "\\":
                out.append(b[i])
                i += 1
                continue
            i += 1
            if i >= len(b):
                break
            e = chr(b[i])
            if e == "n":
                out.append(10)
            elif e == "r":
                out.append(13)
            elif e == "t":
                out.append(9)
            elif e == "\\":
                out.append(92)
            elif e == '"':
                out.append(34)
            elif e == "x" and i + 2 < len(b):
                h = bytes([b[i + 1], b[i + 2]]).decode()
                try:
                    out.append(int(h, 16))
                    i += 2
                except ValueError:
                    out.append(ord("x"))
            else:
                out.append(ord(e))
            i += 1
    return out.decode("utf-8", "replace")


def write_jsonl(name: str, rows):
    p = OUT / name
    with p.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")


def main():
    ai_src = (REF / "src/c-lib/ai.c").read_text()
    ai_rows = []
    for call in extract_calls(ai_src, "test_parseAIdata"):
        args = split_args(call)
        if len(args) != 3:
            continue
        if not args[1].lstrip().startswith('"') or not args[2].lstrip().startswith('"'):
            continue
        ai_rows.append(
            {
                "should_succeed": args[0] == "true",
                "input": decode_c_expr(args[1]),
                "expected": decode_c_expr(args[2]),
            }
        )

    sc_src = (REF / "src/c-lib/scandata.c").read_text()
    sc_rows = []
    for call in extract_calls(sc_src, "test_testProcessScanData"):
        args = split_args(call)
        if len(args) != 4:
            continue
        if not args[1].lstrip().startswith('"') or not args[3].lstrip().startswith('"'):
            continue
        sc_rows.append(
            {
                "should_succeed": args[0] == "true",
                "scan_data": decode_c_expr(args[1]),
                "expected_sym": args[2],
                "expected_data": decode_c_expr(args[3]),
            }
        )

    write_jsonl("ai_parse.jsonl", ai_rows)
    write_jsonl("scandata_process.jsonl", sc_rows)
    dl_src = (REF / "src/c-lib/dl.c").read_text()
    dl_calls = extract_calls(dl_src, "test_parseDLuri")
    dl_rows = []
    cursor = 0
    permit_convenience_alphas = False
    permit_zero_suppressed_gtin = False
    permit_unknown_ais = False
    validate_unknown_ai_not_dl_attr = True
    for call in dl_calls:
        pos = dl_src.find(call, cursor)
        if pos >= 0:
            prelude = dl_src[cursor:pos]
            if "ctx->permitConvenienceAlphas = true" in prelude:
                permit_convenience_alphas = True
            if "ctx->permitConvenienceAlphas = false" in prelude:
                permit_convenience_alphas = False
            if "gs1_encoder_setPermitZeroSuppressedGTINinDLuris(ctx, true)" in prelude:
                permit_zero_suppressed_gtin = True
            if "gs1_encoder_setPermitZeroSuppressedGTINinDLuris(ctx, false)" in prelude:
                permit_zero_suppressed_gtin = False
            if "gs1_encoder_setPermitUnknownAIs(ctx, true)" in prelude:
                permit_unknown_ais = True
            if "gs1_encoder_setPermitUnknownAIs(ctx, false)" in prelude:
                permit_unknown_ais = False
            if "gs1_encoder_setValidationEnabled(ctx, gs1_encoder_vUNKNOWN_AI_NOT_DL_ATTR, false)" in prelude:
                validate_unknown_ai_not_dl_attr = False
            if "gs1_encoder_setValidationEnabled(ctx, gs1_encoder_vUNKNOWN_AI_NOT_DL_ATTR, true)" in prelude:
                validate_unknown_ai_not_dl_attr = True
            cursor = pos + len(call)
        args = split_args(call)
        if len(args) != 3:
            continue
        if not args[1].lstrip().startswith('"') or not args[2].lstrip().startswith('"'):
            continue
        dl_rows.append(
            {
                "should_succeed": args[0] == "true",
                "input": decode_c_expr(args[1]),
                "expected": decode_c_expr(args[2]),
                "permit_convenience_alphas": permit_convenience_alphas,
                "permit_zero_suppressed_gtin": permit_zero_suppressed_gtin,
                "permit_unknown_ais": permit_unknown_ais,
                "validate_unknown_ai_not_dl_attr": validate_unknown_ai_not_dl_attr,
            }
        )
    write_jsonl("dl_parse.jsonl", dl_rows)

    print(f"wrote ai_parse.jsonl rows={len(ai_rows)}")
    print(f"wrote scandata_process.jsonl rows={len(sc_rows)}")
    print(f"wrote dl_parse.jsonl rows={len(dl_rows)}")


if __name__ == "__main__":
    main()
