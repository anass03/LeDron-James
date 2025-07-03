/// Delegation for functions that borrow self immutably
#[macro_export]
macro_rules! delegate_to_host_ref {
    (
        $target:ident => {
            $(
                $(#[$meta:meta])*
                $vis:vis fn $name:ident(&self $(, $arg:ident : $typ:ty )* ) $(-> $ret:ty)? ;
            )*
        }
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name(&self $(, $arg : $typ)* ) $(-> $ret)? {
                self.$target.host.$name($($arg),*)
            }
        )*
    };
}

/// Delegation for functions that borrow self mutably
#[macro_export]
macro_rules! delegate_to_host_mut {
    (
        $target:ident => {
            $(
                $(#[$meta:meta])*
                $vis:vis fn $name:ident(&mut self $(, $arg:ident : $typ:ty )* ) $(-> $ret:ty)? ;
            )*
        }
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name(&mut self $(, $arg : $typ)* ) $(-> $ret)? {
                self.$target.host.$name($($arg),*)
            }
        )*
    };
}
#[macro_export]
macro_rules! delegate_genserver_host_ref {
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $name:ident(&self $(, $arg:ident : $typ:ty )* ) $(-> $ret:ty)? ;
        )*
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name(&self $(, $arg : $typ)* ) $(-> $ret)? {
                self.host.$name($($arg),*)
            }
        )*
    };
}
