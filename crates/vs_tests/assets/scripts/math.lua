-- A visual script. bevy-editor generated this file from the graph below.
-- Edit the graph in the editor: changes to the code are replaced on save.
--[==[bevy-visual-script
{
  "version": 1,
  "next_id": 33,
  "variables": [
    {
      "name": "phase",
      "value": 0.5
    }
  ],
  "nodes": [
    {
      "id": 1,
      "kind": "event.update",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 2,
      "kind": "time.elapsed",
      "x": 0.0,
      "y": 200.0
    },
    {
      "id": 3,
      "kind": "var.get",
      "x": 0.0,
      "y": 300.0,
      "var": "phase"
    },
    {
      "id": 4,
      "kind": "math.add",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 5,
      "kind": "math.sin",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0
      }
    },
    {
      "id": 6,
      "kind": "math.abs",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0
      }
    },
    {
      "id": 7,
      "kind": "math.mul",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 3.0
      }
    },
    {
      "id": 8,
      "kind": "math.min",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 2.0,
        "b": 0.0
      }
    },
    {
      "id": 9,
      "kind": "math.cos",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0
      }
    },
    {
      "id": 10,
      "kind": "math.max",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": -0.5
      }
    },
    {
      "id": 11,
      "kind": "math.div",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 4.0
      }
    },
    {
      "id": 12,
      "kind": "math.sub",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 13,
      "kind": "math.number",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": 1.5
      }
    },
    {
      "id": 14,
      "kind": "math.sin",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.25
      }
    },
    {
      "id": 15,
      "kind": "math.mul",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 16,
      "kind": "math.sub",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 1.0,
        "b": 0.0
      }
    },
    {
      "id": 17,
      "kind": "transform.set_position",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 18,
      "kind": "transform.rotate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 45.0
      }
    },
    {
      "id": 19,
      "kind": "logic.gt",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 1.0
      }
    },
    {
      "id": 20,
      "kind": "logic.lt",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.5
      }
    },
    {
      "id": 21,
      "kind": "logic.or",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": false,
        "b": false
      }
    },
    {
      "id": 22,
      "kind": "logic.eq",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 23,
      "kind": "logic.not",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": false
      }
    },
    {
      "id": 24,
      "kind": "logic.and",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": false,
        "b": false
      }
    },
    {
      "id": 25,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 26,
      "kind": "transform.set_scale",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 1.0,
        "y": 1.0,
        "z": 1.0
      }
    },
    {
      "id": 27,
      "kind": "transform.set_scale",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.5,
        "y": 0.5,
        "z": 0.5
      }
    },
    {
      "id": 28,
      "kind": "event.key",
      "x": 0.0,
      "y": 600.0,
      "values": {
        "key": "Space"
      }
    },
    {
      "id": 29,
      "kind": "text.from_bool",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": false
      }
    },
    {
      "id": 30,
      "kind": "text.from_number",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": 0.0
      }
    },
    {
      "id": 31,
      "kind": "text.join",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": "",
        "b": ""
      }
    },
    {
      "id": 32,
      "kind": "action.print",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": "Hello"
      }
    }
  ],
  "links": [
    {
      "from": {
        "node": 2,
        "pin": "seconds"
      },
      "to": {
        "node": 4,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 3,
        "pin": "value"
      },
      "to": {
        "node": 4,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 4,
        "pin": "result"
      },
      "to": {
        "node": 5,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 5,
        "pin": "result"
      },
      "to": {
        "node": 6,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 6,
        "pin": "result"
      },
      "to": {
        "node": 7,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 7,
        "pin": "result"
      },
      "to": {
        "node": 8,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 2,
        "pin": "seconds"
      },
      "to": {
        "node": 9,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 9,
        "pin": "result"
      },
      "to": {
        "node": 10,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 10,
        "pin": "result"
      },
      "to": {
        "node": 11,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 8,
        "pin": "result"
      },
      "to": {
        "node": 12,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 11,
        "pin": "result"
      },
      "to": {
        "node": 12,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 13,
        "pin": "result"
      },
      "to": {
        "node": 15,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 14,
        "pin": "result"
      },
      "to": {
        "node": 15,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 2,
        "pin": "seconds"
      },
      "to": {
        "node": 16,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 1,
        "pin": "then"
      },
      "to": {
        "node": 17,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 12,
        "pin": "result"
      },
      "to": {
        "node": 17,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 15,
        "pin": "result"
      },
      "to": {
        "node": 17,
        "pin": "y"
      }
    },
    {
      "from": {
        "node": 16,
        "pin": "result"
      },
      "to": {
        "node": 17,
        "pin": "z"
      }
    },
    {
      "from": {
        "node": 17,
        "pin": "then"
      },
      "to": {
        "node": 18,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 16,
        "pin": "result"
      },
      "to": {
        "node": 18,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 2,
        "pin": "seconds"
      },
      "to": {
        "node": 19,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 2,
        "pin": "seconds"
      },
      "to": {
        "node": 20,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 19,
        "pin": "result"
      },
      "to": {
        "node": 21,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 20,
        "pin": "result"
      },
      "to": {
        "node": 21,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 12,
        "pin": "result"
      },
      "to": {
        "node": 22,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 22,
        "pin": "result"
      },
      "to": {
        "node": 23,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 21,
        "pin": "result"
      },
      "to": {
        "node": 24,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 23,
        "pin": "result"
      },
      "to": {
        "node": 24,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 18,
        "pin": "then"
      },
      "to": {
        "node": 25,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 24,
        "pin": "result"
      },
      "to": {
        "node": 25,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 25,
        "pin": "true"
      },
      "to": {
        "node": 26,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 16,
        "pin": "result"
      },
      "to": {
        "node": 26,
        "pin": "y"
      }
    },
    {
      "from": {
        "node": 25,
        "pin": "false"
      },
      "to": {
        "node": 27,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 24,
        "pin": "result"
      },
      "to": {
        "node": 29,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 12,
        "pin": "result"
      },
      "to": {
        "node": 30,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 29,
        "pin": "result"
      },
      "to": {
        "node": 31,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 30,
        "pin": "result"
      },
      "to": {
        "node": 31,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 28,
        "pin": "then"
      },
      "to": {
        "node": 32,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 31,
        "pin": "result"
      },
      "to": {
        "node": 32,
        "pin": "value"
      }
    }
  ]
}
]==]

local vars = {
    0.5, -- phase
}
local state = {}

function on_update()
    -- On Update [1]
    -- Set Position [17]
    do
        local t0 = node.math.add(vs.time(), vars[1])
        local t1 = node.math.sin(t0)
        local t2 = node.math.abs(t1)
        local t3 = node.math.mul(t2, 3)
        local t4 = node.math.min(2, t3)
        local t5 = node.math.cos(vs.time())
        local t6 = node.math.max(t5, -0.5)
        local t7 = node.math.div(t6, 4)
        local t8 = node.math.sub(t4, t7)
        local t9 = node.math.number(1.5)
        local t10 = node.math.sin(0.25)
        local t11 = node.math.mul(t9, t10)
        local t12 = node.math.sub(1, vs.time())
        vs.set_position(entity, t8, t11, t12)
    end
    -- Rotate [18]
    do
        local t13 = node.math.sub(1, vs.time())
        vs.rotate(entity, t13, 0, 45)
    end
    -- Branch [25]
    do
        local t14 = node.logic.gt(vs.time(), 1)
        local t15 = node.logic.lt(vs.time(), 0.5)
        local t16 = node.logic["or"](t14, t15)
        local t17 = node.math.add(vs.time(), vars[1])
        local t18 = node.math.sin(t17)
        local t19 = node.math.abs(t18)
        local t20 = node.math.mul(t19, 3)
        local t21 = node.math.min(2, t20)
        local t22 = node.math.cos(vs.time())
        local t23 = node.math.max(t22, -0.5)
        local t24 = node.math.div(t23, 4)
        local t25 = node.math.sub(t21, t24)
        local t26 = node.logic.eq(t25, 0)
        local t27 = node.logic["not"](t26)
        local t28 = node.logic["and"](t16, t27)
        if t28 then
            -- Set Scale [26]
            do
                local t29 = node.math.sub(1, vs.time())
                vs.set_scale(entity, 1, t29, 1)
            end
        else
            -- Set Scale [27]
            vs.set_scale(entity, 0.5, 0.5, 0.5)
        end
    end
    -- On Key Pressed [28]
    if vs.input("key", "pressed", "Space") then
        -- Print [32]
        do
            local t30 = node.logic.gt(vs.time(), 1)
            local t31 = node.logic.lt(vs.time(), 0.5)
            local t32 = node.logic["or"](t30, t31)
            local t33 = node.math.add(vs.time(), vars[1])
            local t34 = node.math.sin(t33)
            local t35 = node.math.abs(t34)
            local t36 = node.math.mul(t35, 3)
            local t37 = node.math.min(2, t36)
            local t38 = node.math.cos(vs.time())
            local t39 = node.math.max(t38, -0.5)
            local t40 = node.math.div(t39, 4)
            local t41 = node.math.sub(t37, t40)
            local t42 = node.logic.eq(t41, 0)
            local t43 = node.logic["not"](t42)
            local t44 = node.logic["and"](t32, t43)
            local t45 = node.text.from_bool(t44)
            local t46 = node.text.from_number(t41)
            local t47 = node.text.join(t45, t46)
            print(t47)
        end
    end
end
