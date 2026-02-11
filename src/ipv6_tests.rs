/*
 * MIT License
 *
 * Copyright (c) 2025 Julian Kahlert
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

#[cfg(test)]
mod tests {
    use crate::resolve_socket_addr;

    #[test]
    fn test_ipv6_link_local_with_scope() {
        let result = resolve_socket_addr("fe80::1", 22, Some("lo"));
        match result {
            Ok(addr) => {
                assert!(addr.is_ipv6());
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to resolve host")
                    || err_msg.contains("No socket addresses resolved for")
                {
                } else {
                    panic!("Unexpected error: {}", err_msg);
                }
            }
        }
    }

    #[test]
    fn test_ipv6_link_local_without_scope() {
        let result = resolve_socket_addr("fe80::1", 22, None);
        match result {
            Ok(addr) => {
                assert!(addr.is_ipv6());
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to resolve host")
                    || err_msg.contains("No socket addresses resolved for")
                {
                } else {
                    panic!("Unexpected error: {}", err_msg);
                }
            }
        }
    }

    #[test]
    fn test_ipv6_global_address() {
        let result = resolve_socket_addr("2001:db8::1", 22, None);
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert!(addr.is_ipv6());
    }

    #[test]
    fn test_ipv4_address() {
        let result = resolve_socket_addr("192.168.1.1", 22, None);
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert!(addr.is_ipv4());
    }

    #[test]
    fn test_scope_with_numeric_id() {
        let result = resolve_socket_addr("fe80::1", 22, Some("1"));
        match result {
            Ok(addr) => {
                assert!(addr.is_ipv6());
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Failed to resolve host")
                    || err_msg.contains("No socket addresses resolved for")
                {
                } else {
                    panic!("Unexpected error: {}", err_msg);
                }
            }
        }
    }

    #[test]
    fn test_invalid_address() {
        let result = resolve_socket_addr("not-a-valid-address", 22, None);
        assert!(result.is_err());
    }
}
