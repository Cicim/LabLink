pub(crate) struct Machine {
    pub name: &'static str,
    pub url: &'static str,
}

pub(crate) static MACHINES: &[Machine] = &[
    Machine {
        name: "Controls",
        url: "http://192.168.20.250:80",
    },
    Machine {
        name: "Cermac",
        url: "http://192.168.20.251:80",
    },
];
