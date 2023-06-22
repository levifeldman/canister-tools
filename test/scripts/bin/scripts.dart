import 'dart:typed_data';
import 'dart:io';
import 'dart:math';

import 'package:ic_tools/ic_tools.dart';
import 'package:ic_tools/common.dart';
import 'package:ic_tools/tools.dart';
import 'package:ic_tools/candid.dart';
import 'package:msgpack_dart/msgpack_dart.dart';



const int data_upgrade_serialization_memory_id = 0;

const int chunk_size = 1024 * 512 * 3;

Caller caller = Caller(keys: Ed25519Keys.new_keys());


Future<void> main(List<String> arguments) async {
    ic_base_url = Uri.parse('http://127.0.0.1:8080');    
    await fetch_root_key();
        
    Principal canister_id = await provisional_create_canister_with_cycles(caller:caller, cycles: BigInt.from(500000000000000));
    print(canister_id);
    
    await put_code_on_the_canister(
        caller,
        canister_id,
        File('../canister/target/wasm32-unknown-unknown/release/test_canister.wasm').readAsBytesSync(),
        CanisterInstallMode.install
    );
        
    Uint8List state_snapshot = await create_and_download_state_snapshot(canister_id);
    
    await load_state_snapshot(canister_id, state_snapshot);
    
    Uint8List state_snapshot_2 = await create_and_download_state_snapshot(canister_id);
    
    if (aresamebytes(state_snapshot, state_snapshot_2) == false) {
        throw Exception('check this');
    }
    
    int get_field_two = (c_backwards_one(await Canister(canister_id).call(
        calltype: CallType.query,
        method_name: 'get_field_two',
    )) as Nat64).value.toInt(); 
    
    dynamic msgpack_backwards = deserialize(await create_and_download_state_snapshot(canister_id));
    
    if (msgpack_backwards[1] != 55 || 55 != get_field_two) {
        throw Exception('check this');
    }
    
    msgpack_backwards[1] = 200321321;
    
    await load_state_snapshot(canister_id, serialize(msgpack_backwards));
    
    get_field_two = (c_backwards_one(await Canister(canister_id).call(
        calltype: CallType.query,
        method_name: 'get_field_two',
    )) as Nat64).value.toInt(); 
    
    if (get_field_two != 200321321) {
        throw Exception('check this');
    }
    
    await Canister(canister_id).call(
        calltype: CallType.call,
        method_name: 'set_field_two',
        put_bytes: c_forwards_one(Nat64(BigInt.from(102154646898)))
    ); 
    
    Uint8List snapshot = await create_and_download_state_snapshot(canister_id);
    msgpack_backwards = deserialize(snapshot);
    
    if (msgpack_backwards[1] != 102154646898) {
        throw Exception('check this');
    }
    
    await put_code_on_the_canister(
        caller,
        canister_id,
        File('../canister/target/wasm32-unknown-unknown/release/test_canister.wasm').readAsBytesSync(),
        CanisterInstallMode.upgrade
    );
    
    if (aresamebytes(snapshot, await create_and_download_state_snapshot(canister_id)) == false) {
        throw Exception('check this');
    }
    
    
    
    
}



Future<Uint8List> create_and_download_state_snapshot(Principal canister_id) async {
    
    int snapshot_length = (c_backwards_one(await Canister(canister_id).call(
        method_name: 'controller_create_state_snapshot',
        calltype: CallType.call,
        put_bytes: c_forwards_one(Nat8(data_upgrade_serialization_memory_id)),    
        caller:caller,
    )) as Nat64).value.toInt();
    
    List<int> snapshot = [];
    while (snapshot.length < snapshot_length) {
        snapshot.addAll(
            (c_backwards_one(await Canister(canister_id).call(
                method_name: 'controller_download_state_snapshot',
                caller:caller,
                calltype: CallType.query,
                put_bytes: c_forwards([
                    Nat8(data_upgrade_serialization_memory_id),
                    Nat64(BigInt.from(snapshot.length)),
                    Nat64(BigInt.from(min(chunk_size, snapshot_length - snapshot.length)))
                ])
            )) as Blob).bytes    
        );
    }
    
    return Uint8List.fromList(snapshot); 

}

Future<void> load_state_snapshot(Principal canister_id, Uint8List load_snapshot) async {
    
    await Canister(canister_id).call(
        calltype: CallType.call,
        method_name: 'controller_clear_state_snapshot',
        put_bytes: c_forwards_one(Nat8(data_upgrade_serialization_memory_id)),
        caller: caller
    );
    
    List<Uint8List> chunks = load_snapshot.chunks(chunk_size);
    for (Uint8List chunk in chunks) {
        await Canister(canister_id).call(
            calltype: CallType.call,
            method_name: 'controller_append_state_snapshot',
            put_bytes: c_forwards([
                Nat8(data_upgrade_serialization_memory_id),
                Blob(chunk)
            ]),
            caller: caller
        );
    }
    
    await Canister(canister_id).call(
        calltype: CallType.call,
        method_name: 'controller_load_state_snapshot',
        put_bytes: c_forwards_one(Nat8(data_upgrade_serialization_memory_id)),
        caller: caller
    );
       
}






extension Chunks<T extends List> on T {
    List<T> chunks(int chunk_size) {
        var b_len = this.length;
        List<T> chunks = [];
        for(int i = 0; i < b_len; i += chunk_size) {    
            chunks.add(this.sublist(i,min(i+chunk_size, b_len)) as T);
        }
        return chunks;
    }
} 