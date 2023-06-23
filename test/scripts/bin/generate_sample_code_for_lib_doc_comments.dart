import 'dart:io';

void main(List<String> arguments) {
    List<String> file_lines = File('../canister/src/lib.rs').readAsLinesSync();
    String generate = '//! ${file_lines.join('\n//! ')}';
    File('lib_doc_sample').writeAsStringSync(generate);
}
