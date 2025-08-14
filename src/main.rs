fn main() {
    println!("Hello, world!");
    // input:
    // which port should i run on?
    //
    // apis:
    //   register:
    //    node address, node topics, node name
    //     store in sled db, keyed on node topic?
    //   topic_info:
    //    topic name in, node address out
    //    get info from sled db
    //
    // client then goes and talks directly to bob-ka node it cares about
    // if location of node changes, or topic moves from that node?
    // how does client refresh its knowledge of where the node is?
    // this is likely a future problem, don't want to get to bogged down here
}
