impl ::std::clone::Clone for l2cap_conf_req {
    fn clone(&self) -> Self {
        panic!("l2cap_conf_req cannot implement Clone/Copy safely!");
    }
}

impl ::std::clone::Clone for l2cap_conf_rsp {
    fn clone(&self) -> Self {
        panic!("l2cap_conf_rsp cannot implement Clone/Copy safely!");
    }
}

impl ::std::clone::Clone for l2cap_info_rsp {
    fn clone(&self) -> Self {
        panic!("l2cap_info_rsp cannot implement Clone/Copy safely!");
    }
}

impl ::std::marker::Copy for l2cap_conf_req {}
impl ::std::marker::Copy for l2cap_conf_rsp {}
impl ::std::marker::Copy for l2cap_info_rsp {}
