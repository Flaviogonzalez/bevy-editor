-- A visual script. bevy-editor generated this file from the graph below.
-- Edit the graph in the editor: changes to the code are replaced on save.
--[==[bevy-visual-script
{
  "version": 1,
  "next_id": 29,
  "variables": [
    {
      "name": "frames",
      "value": 0.0
    },
    {
      "name": "label",
      "value": "frames: "
    },
    {
      "name": "moved",
      "value": false
    }
  ],
  "nodes": [
    {
      "id": 1,
      "kind": "event.start",
      "x": 0.0,
      "y": -200.0
    },
    {
      "id": 2,
      "kind": "flow.do_once",
      "x": 260.0,
      "y": -200.0
    },
    {
      "id": 3,
      "kind": "action.print",
      "x": 520.0,
      "y": -200.0,
      "values": {
        "value": "logic script loaded: W moves, it resets every 100 frames"
      }
    },
    {
      "id": 4,
      "kind": "event.update",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 5,
      "kind": "flow.sequence",
      "x": 260.0,
      "y": 0.0
    },
    {
      "id": 6,
      "kind": "var.get",
      "x": 260.0,
      "y": 140.0,
      "var": "frames"
    },
    {
      "id": 7,
      "kind": "math.add",
      "x": 520.0,
      "y": 140.0,
      "values": {
        "a": 0.0,
        "b": 1.0
      }
    },
    {
      "id": 8,
      "kind": "var.set",
      "x": 780.0,
      "y": 0.0,
      "values": {
        "value": 0.0
      },
      "var": "frames"
    },
    {
      "id": 9,
      "kind": "logic.gt",
      "x": 1040.0,
      "y": 140.0,
      "values": {
        "a": 0.0,
        "b": 99.0
      }
    },
    {
      "id": 10,
      "kind": "flow.branch",
      "x": 1040.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 11,
      "kind": "var.get",
      "x": 1300.0,
      "y": 260.0,
      "var": "label"
    },
    {
      "id": 12,
      "kind": "text.join",
      "x": 1560.0,
      "y": 260.0,
      "values": {
        "a": "",
        "b": ""
      }
    },
    {
      "id": 13,
      "kind": "action.print",
      "x": 1300.0,
      "y": -60.0,
      "values": {
        "value": "Hello"
      }
    },
    {
      "id": 14,
      "kind": "transform.position",
      "x": 1560.0,
      "y": 120.0
    },
    {
      "id": 15,
      "kind": "transform.set_position",
      "x": 1560.0,
      "y": -60.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 16,
      "kind": "var.set",
      "x": 1820.0,
      "y": -60.0,
      "values": {
        "value": 0.0
      },
      "var": "frames"
    },
    {
      "id": 17,
      "kind": "input.key_down",
      "x": 260.0,
      "y": 400.0,
      "values": {
        "key": "KeyW"
      }
    },
    {
      "id": 18,
      "kind": "flow.branch",
      "x": 520.0,
      "y": 320.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 19,
      "kind": "math.mul",
      "x": 780.0,
      "y": 420.0,
      "values": {
        "a": 0.0,
        "b": -3.0
      }
    },
    {
      "id": 20,
      "kind": "transform.translate",
      "x": 1040.0,
      "y": 320.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 21,
      "kind": "var.set",
      "x": 1300.0,
      "y": 320.0,
      "values": {
        "value": true
      },
      "var": "moved"
    },
    {
      "id": 22,
      "kind": "time.elapsed",
      "x": 780.0,
      "y": 600.0
    },
    {
      "id": 23,
      "kind": "math.sin",
      "x": 1040.0,
      "y": 600.0,
      "values": {
        "x": 0.0
      }
    },
    {
      "id": 24,
      "kind": "math.mul",
      "x": 1300.0,
      "y": 600.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 25,
      "kind": "transform.translate",
      "x": 1560.0,
      "y": 480.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 26,
      "kind": "event.key",
      "x": 0.0,
      "y": 760.0,
      "values": {
        "key": "KeyR"
      }
    },
    {
      "id": 27,
      "kind": "var.get",
      "x": 0.0,
      "y": 880.0,
      "var": "moved"
    },
    {
      "id": 28,
      "kind": "action.print",
      "x": 260.0,
      "y": 760.0,
      "values": {
        "value": "Hello"
      }
    }
  ],
  "links": [
    {
      "from": {
        "node": 1,
        "pin": "then"
      },
      "to": {
        "node": 2,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 2,
        "pin": "then"
      },
      "to": {
        "node": 3,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 4,
        "pin": "then"
      },
      "to": {
        "node": 5,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 6,
        "pin": "value"
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
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 5,
        "pin": "first"
      },
      "to": {
        "node": 8,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 8,
        "pin": "value"
      },
      "to": {
        "node": 9,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 9,
        "pin": "result"
      },
      "to": {
        "node": 10,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 8,
        "pin": "then"
      },
      "to": {
        "node": 10,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 11,
        "pin": "value"
      },
      "to": {
        "node": 12,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 8,
        "pin": "value"
      },
      "to": {
        "node": 12,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 12,
        "pin": "result"
      },
      "to": {
        "node": 13,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 10,
        "pin": "true"
      },
      "to": {
        "node": 13,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 14,
        "pin": "y"
      },
      "to": {
        "node": 15,
        "pin": "y"
      }
    },
    {
      "from": {
        "node": 13,
        "pin": "then"
      },
      "to": {
        "node": 15,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 15,
        "pin": "then"
      },
      "to": {
        "node": 16,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 17,
        "pin": "down"
      },
      "to": {
        "node": 18,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 5,
        "pin": "second"
      },
      "to": {
        "node": 18,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 4,
        "pin": "delta"
      },
      "to": {
        "node": 19,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 19,
        "pin": "result"
      },
      "to": {
        "node": 20,
        "pin": "z"
      }
    },
    {
      "from": {
        "node": 18,
        "pin": "true"
      },
      "to": {
        "node": 20,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 20,
        "pin": "then"
      },
      "to": {
        "node": 21,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 22,
        "pin": "seconds"
      },
      "to": {
        "node": 23,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 23,
        "pin": "result"
      },
      "to": {
        "node": 24,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 4,
        "pin": "delta"
      },
      "to": {
        "node": 24,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 24,
        "pin": "result"
      },
      "to": {
        "node": 25,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 18,
        "pin": "false"
      },
      "to": {
        "node": 25,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 27,
        "pin": "value"
      },
      "to": {
        "node": 28,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 26,
        "pin": "then"
      },
      "to": {
        "node": 28,
        "pin": "in"
      }
    }
  ]
}
]==]

local vars = {
    0, -- frames
    "frames: ", -- label
    false, -- moved
}
local state = { false }

function on_script_loaded()
    -- On Start [1]
    -- Do Once [2]
    do
        local t0 = node.logic["not"](state[1])
        if t0 then
            state[1] = true
            -- Print [3]
            print("logic script loaded: W moves, it resets every 100 frames")
        end
    end
end

function on_update()
    -- On Update [4]
    -- Set frames [8]
    do
        local t1 = node.math.add(vars[1], 1)
        vars[1] = t1
    end
    -- Branch [10]
    do
        local t2 = node.logic.gt(vars[1], 99)
        if t2 then
            -- Print [13]
            do
                local t3 = node.text.from_number(vars[1])
                local t4 = node.text.join(vars[2], t3)
                print(t4)
            end
            -- Set Position [15]
            vs.set_position(entity, 0, vs.position(entity).y, 0)
            -- Set frames [16]
            vars[1] = 0
        end
    end
    -- Branch [18]
    if vs.input("key", "down", "KeyW") then
        -- Translate [20]
        do
            local t5 = node.math.mul(vs.delta(), -3)
            vs.translate(entity, 0, 0, t5)
        end
        -- Set moved [21]
        vars[3] = true
    else
        -- Translate [25]
        do
            local t6 = node.math.sin(vs.time())
            local t7 = node.math.mul(t6, vs.delta())
            vs.translate(entity, t7, 0, 0)
        end
    end
    -- On Key Pressed [26]
    if vs.input("key", "pressed", "KeyR") then
        -- Print [28]
        do
            local t8 = node.text.from_bool(vars[3])
            print(t8)
        end
    end
end
