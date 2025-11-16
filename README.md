# 使用Rust编写的系统资源工具
工具会将系统使用HTTP发送到服务器

## 请求结构
*请求方式：POST*

**json请求体**
| 字段    | 类型  | 内容             |
| ------- | ----- | ---------------- |
| cpu     | array | 每个核心cpu占用率 |
| mem     | obj   | 内存占用          |
| swap    | obj   | swap占用         |
| net     | obj   | 网卡及流量        |
| proc    | obj   | 进程数量          |

--- 

mem与swap对象:
| 字段 | 类型 |
| ---- | ---- |
| total | num |
| used | num |

单位为字节

--- 

net对象的字段为网卡名
子对象:
| 字段 | 类型 |
| ---- | ---- |
| rx | num |
| tx | num |

单位为字节

--- 


proc对象:
| 字段 | 类型 |
| ---- | ---- |
| total | num |
| running | num |
| sleeping | num |
| zombie | num |

---

```json
{
    "cpu": [
        2.857143,
        1.904762,
        1.923077,
        4.7169814
    ],
    "mem": {
        "total": 16360284160,
        "used": 10183102464
    },
    "swap": {
        "total": 17179865088,
        "used": 4194304
    },
    "net": {
        "enp3s0": {
            "rx": 1423,
            "tx": 3449
        },
        "vetha192bc7": {
            "rx": 0,
            "tx": 0
        },
        "veth5c3bd97": {
            "rx": 0,
            "tx": 0
        },
        "lo": {
            "rx": 4094,
            "tx": 4094
        },
        "docker0": {
            "rx": 0,
            "tx": 0
        }
    },
    "proc": {
        "total": 280,
        "running": 0,
        "sleeping": 215,
        "zombie": 0
    }
}
```
