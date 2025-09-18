pub mod user {
    tonic::include_proto!("accountcat.user");
}
pub mod todolist {
    tonic::include_proto!("accountcat.todolist");
}

pub mod accounting {
    tonic::include_proto!("accountcat.accounting");
}

pub mod instance_setting {
    tonic::include_proto!("accountcat.instance_setting");
}
