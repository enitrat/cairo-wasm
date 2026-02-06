#!/usr/bin/env sh

# move to the directory where the script is located
cd $(dirname "$0")

tests_path="../../../cairo_programs/"
exit_code=0
passed_tests=0
failed_tests=0

files=$(ls $tests_path)
EXIT_CODE=$?
if [ ${EXIT_CODE} != 0 ]; then
    exit ${EXIT_CODE}
fi

for file in $(ls $tests_path | grep .rs.pie.zip$ | sed -E 's/\.rs.pie.zip$//'); do
    path_file="$tests_path/$file"

    # Run Cairo PIE using cairo_lang
    echo "Running $file PIE with cairo_lang"
    if ! cairo-run --run_from_cairo_pie $path_file.rs.pie.zip --trace_file $path_file.trace.pie --memory_file $path_file.memory.pie --cairo_pie_output $path_file.pie.zip.pie --layout starknet_with_keccak 2> /dev/null; then
        echo "Skipping $file.pie as it fails validations in cairo_lang"
        break
    fi
    echo "Running $file PIE with cairo-vm"
     cargo run -p cairo-vm-cli --release $path_file.rs.pie.zip --run_from_cairo_pie  --trace_file $path_file.rs.trace.pie --memory_file $path_file.rs.memory.pie --cairo_pie_output $path_file.rs.pie.zip.pie --layout starknet_with_keccak --fill-holes false
    # Compare PIE outputs
    echo "Comparing $file.pie outputs"

    # Compare trace
    if ! diff -q $path_file.trace.pie $path_file.rs.trace.pie; then
        echo "Traces for $file differ"
        exit_code=1
        failed_tests=$((failed_tests + 1))
    else
        passed_tests=$((passed_tests + 1))
    fi

    # Compare Memory
    if ! ./memory_comparator.py $path_file.memory.pie $path_file.rs.memory.pie; then
        echo "Memory differs for $file"
        exit_code=1
        failed_tests=$((failed_tests + 1))
    else
        passed_tests=$((passed_tests + 1))
    fi

    # Compare PIE
    if ! ./cairo_pie_comparator.py $path_file.pie.zip.pie $path_file.rs.pie.zip.pie; then
        echo "Cairo PIE differs for $file"
        exit_code=1
        failed_tests=$((failed_tests + 1))
    else
        passed_tests=$((passed_tests + 1))
    fi
done

if test $failed_tests != 0; then
    echo "Comparisons: $failed_tests failed, $passed_tests passed, $((failed_tests + passed_tests)) total"
elif test $passed_tests = 0; then
    echo "No tests ran!"
    exit_code=2
else
    echo "All $passed_tests tests passed; no discrepancies found"
fi

exit "${exit_code}"
