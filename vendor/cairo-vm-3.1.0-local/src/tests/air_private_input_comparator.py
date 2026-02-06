#!/usr/bin/env python3

import sys
import json


filename1 = sys.argv[1]
filename2 = sys.argv[2]

with open(filename1, 'r') as cairo_lang_input_file, open(filename2, 'r') as cairo_vm_input_file:
    cairo_lang_input = json.load(cairo_lang_input_file)
    cairo_vm_input = json.load(cairo_vm_input_file)
    # The trace_path & memory_path fields contain the path of each file which will differ 
    # as we use a different extension to differentiate between python & rust vm outputs
    cairo_lang_input["trace_path"] = ""
    cairo_lang_input["memory_path"] = ""

    cairo_vm_input["trace_path"] = ""
    cairo_vm_input["memory_path"] = ""

    if cairo_lang_input == cairo_vm_input:
    
        print(f"Comparison succesful for {filename1} vs {filename2}")
    else:
        print(f"Comparison unsuccesful for {filename1} vs {filename2}")
        exit(1)
