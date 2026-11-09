-- A visual script. bevy-editor generated this file from the graph below.
-- Edit the graph in the editor: changes to the code are replaced on save.
--[==[bevy-visual-script
{
  "version": 1,
  "next_id": 17,
  "variables": [
    {
      "name": "speed",
      "value": 90.0
    }
  ],
  "nodes": [
    {
      "id": 1,
      "kind": "event.start",
      "x": 0.0,
      "y": -160.0
    },
    {
      "id": 2,
      "kind": "action.print",
      "x": 280.0,
      "y": -160.0,
      "values": {
        "value": "demo script loaded: Space toggles the size"
      }
    },
    {
      "id": 3,
      "kind": "event.update",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 4,
      "kind": "var.get",
      "x": 0.0,
      "y": 120.0,
      "var": "speed"
    },
    {
      "id": 5,
      "kind": "math.mul",
      "x": 260.0,
      "y": 100.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 6,
      "kind": "transform.rotate",
      "x": 540.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 7,
      "kind": "time.elapsed",
      "x": 0.0,
      "y": 260.0
    },
    {
      "id": 8,
      "kind": "math.mul",
      "x": 260.0,
      "y": 240.0,
      "values": {
        "a": 0.0,
        "b": 2.0
      }
    },
    {
      "id": 9,
      "kind": "math.sin",
      "x": 520.0,
      "y": 240.0,
      "values": {
        "x": 0.0
      }
    },
    {
      "id": 10,
      "kind": "math.mul",
      "x": 780.0,
      "y": 240.0,
      "values": {
        "a": 0.0,
        "b": 0.25
      }
    },
    {
      "id": 11,
      "kind": "math.add",
      "x": 1040.0,
      "y": 240.0,
      "values": {
        "a": 0.0,
        "b": 1.0
      }
    },
    {
      "id": 12,
      "kind": "transform.set_position",
      "x": 1300.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 13,
      "kind": "event.key",
      "x": 0.0,
      "y": 420.0,
      "values": {
        "key": "Space"
      }
    },
    {
      "id": 14,
      "kind": "flow.flipflop",
      "x": 280.0,
      "y": 420.0
    },
    {
      "id": 15,
      "kind": "transform.set_scale",
      "x": 560.0,
      "y": 380.0,
      "values": {
        "x": 1.6,
        "y": 1.6,
        "z": 1.6
      }
    },
    {
      "id": 16,
      "kind": "transform.set_scale",
      "x": 560.0,
      "y": 520.0,
      "values": {
        "x": 1.0,
        "y": 1.0,
        "z": 1.0
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
        "node": 3,
        "pin": "then"
      },
      "to": {
        "node": 6,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 4,
        "pin": "value"
      },
      "to": {
        "node": 5,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 3,
        "pin": "delta"
      },
      "to": {
        "node": 5,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 5,
        "pin": "result"
      },
      "to": {
        "node": 6,
        "pin": "y"
      }
    },
    {
      "from": {
        "node": 7,
        "pin": "seconds"
      },
      "to": {
        "node": 8,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 8,
        "pin": "result"
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
        "node": 11,
        "pin": "result"
      },
      "to": {
        "node": 12,
        "pin": "y"
      }
    },
    {
      "from": {
        "node": 6,
        "pin": "then"
      },
      "to": {
        "node": 12,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 13,
        "pin": "then"
      },
      "to": {
        "node": 14,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 14,
        "pin": "a"
      },
      "to": {
        "node": 15,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 14,
        "pin": "b"
      },
      "to": {
        "node": 16,
        "pin": "in"
      }
    }
  ]
}
]==]

local vars = {
    ["speed"] = 90,
}
local state = {}

function on_script_loaded()
    -- On Start [1]
    -- Print [2]
    print("demo script loaded: Space toggles the size")
end

function on_update()
    local delta = vs.delta()
    -- On Update [3]
    -- Rotate [6]
    vs.rotate(entity, 0, (vars["speed"] * delta), 0)
    -- Set Position [12]
    vs.set_position(entity, 0, ((math.sin((vs.time() * 2)) * 0.25) + 1), 0)
    -- On Key Pressed [13]
    if vs.key_just_pressed("Space") then
        -- Flip Flop [14]
        state.flipflop_14 = not state.flipflop_14
        if state.flipflop_14 then
            -- Set Scale [15]
            vs.set_scale(entity, 1.6, 1.6, 1.6)
        else
            -- Set Scale [16]
            vs.set_scale(entity, 1, 1, 1)
        end
    end
end
