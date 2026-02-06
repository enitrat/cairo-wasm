#!/usr/bin/env python3

import sys
import json
from zipfile import ZipFile
import memory_comparator

filename1 = sys.argv[1]
filename2 = sys.argv[2]

json_files = ["version.json", "metadata.json", "execution_resources.json", "additional_data.json"]

with ZipFile(filename1) as cairo_lang_pie_zip, ZipFile(filename2) as cairo_vm_pie_zip:
    # Compare json files
    for file in json_files:
        with cairo_lang_pie_zip.open(file) as cairo_lang_file, cairo_vm_pie_zip.open(file) as cairo_vm_file:
            cl_content = json.load(cairo_lang_file)
            cv_content = json.load(cairo_vm_file)
            if cl_content != cv_content:
                print(f"Comparison unsuccesful for {filename1}/{file}  vs {filename2}/{file}")
                exit(1)

    # Compare binary files
    with cairo_lang_pie_zip.open("memory.bin", 'r') as f1,  cairo_vm_pie_zip.open("memory.bin", 'r') as f2:
        memory_comparator.compare_memory_file_contents(f1.read(), f2.read())
