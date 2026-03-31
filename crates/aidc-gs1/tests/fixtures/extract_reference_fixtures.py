#!/usr/bin/env python3
import argparse
import json
import pathlib
import sys

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


def read_jsonl(name: str):
    p = OUT / name
    rows = []
    with p.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def extract_all():
    ai_src = (REF / "src/c-lib/ai.c").read_text()
    sc_src = (REF / "src/c-lib/scandata.c").read_text()
    dl_src = (REF / "src/c-lib/dl.c").read_text()

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

    return {
        "ai_parse": ai_rows,
        "scandata_process": sc_rows,
        "dl_parse": dl_rows,
    }


def write_report(rows_by_name):
    report = {
        "upstream_repo": "https://github.com/gs1/gs1-syntax-engine",
        "source_files": {
            "ai": "src/c-lib/ai.c",
            "scandata": "src/c-lib/scandata.c",
            "dl": "src/c-lib/dl.c",
        },
        "counts": {
            "ai_parse": len(rows_by_name["ai_parse"]),
            "scandata_process": len(rows_by_name["scandata_process"]),
            "dl_parse": len(rows_by_name["dl_parse"]),
        },
        "known_ids": {
            "ai_parse_input": "(01)12345678901231",
            "scandata_sample": "]d2011231231231233310ABC123\\u001d99TESTING",
            "dl_parse_input": "https://a/01/12312312312333",
        },
    }
    p = OUT / "EXTRACTION_REPORT.json"
    p.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


def check_existing(rows_by_name):
    expected = {
        "ai_parse": read_jsonl("ai_parse.jsonl"),
        "scandata_process": read_jsonl("scandata_process.jsonl"),
        "dl_parse": read_jsonl("dl_parse.jsonl"),
    }
    ok = True
    for name in ("ai_parse", "scandata_process", "dl_parse"):
        if rows_by_name[name] != expected[name]:
            print(
                f"fixture mismatch in {name}.jsonl: extracted={len(rows_by_name[name])} existing={len(expected[name])}",
                file=sys.stderr,
            )
            ok = False
    return ok


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify committed fixture JSONL files match extracted upstream calls",
    )
    args = parser.parse_args()

    rows_by_name = extract_all()
    if args.check:
        if not check_existing(rows_by_name):
            return 1
        print(
            "fixtures are in sync: "
            f"ai_parse={len(rows_by_name['ai_parse'])}, "
            f"scandata_process={len(rows_by_name['scandata_process'])}, "
            f"dl_parse={len(rows_by_name['dl_parse'])}"
        )
        return 0

    write_jsonl("ai_parse.jsonl", rows_by_name["ai_parse"])
    write_jsonl("scandata_process.jsonl", rows_by_name["scandata_process"])
    write_jsonl("dl_parse.jsonl", rows_by_name["dl_parse"])
    write_report(rows_by_name)

    print(f"wrote ai_parse.jsonl rows={len(rows_by_name['ai_parse'])}")
    print(f"wrote scandata_process.jsonl rows={len(rows_by_name['scandata_process'])}")
    print(f"wrote dl_parse.jsonl rows={len(rows_by_name['dl_parse'])}")
    print("wrote EXTRACTION_REPORT.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
