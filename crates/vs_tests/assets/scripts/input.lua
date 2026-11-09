-- A visual script. bevy-editor generated this file from the graph below.
-- Edit the graph in the editor: changes to the code are replaced on save.
--[==[bevy-visual-script
{
  "version": 1,
  "next_id": 26,
  "variables": [],
  "nodes": [
    {
      "id": 1,
      "kind": "event.update",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 2,
      "kind": "input.mouse_down",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "button": "Left"
      }
    },
    {
      "id": 3,
      "kind": "flow.sequence",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 4,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 5,
      "kind": "transform.translate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.5,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 6,
      "kind": "input.mouse_pressed",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "button": "Right"
      }
    },
    {
      "id": 7,
      "kind": "flow.sequence",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 8,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 9,
      "kind": "transform.translate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 1.0,
        "z": 0.0
      }
    },
    {
      "id": 10,
      "kind": "input.mouse_released",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "button": "Left"
      }
    },
    {
      "id": 11,
      "kind": "flow.sequence",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 12,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 13,
      "kind": "transform.translate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 1.0
      }
    },
    {
      "id": 14,
      "kind": "input.key_pressed",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "key": "KeyW"
      }
    },
    {
      "id": 15,
      "kind": "flow.sequence",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 16,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 17,
      "kind": "transform.set_scale",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 2.0,
        "y": 1.0,
        "z": 0.5
      }
    },
    {
      "id": 18,
      "kind": "input.key_released",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "key": "Space"
      }
    },
    {
      "id": 19,
      "kind": "flow.sequence",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 20,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 21,
      "kind": "transform.rotate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 10.0,
        "z": 0.0
      }
    },
    {
      "id": 22,
      "kind": "input.key_down",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "key": "KeyR"
      }
    },
    {
      "id": 23,
      "kind": "flow.sequence",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 24,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 25,
      "kind": "transform.translate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": -0.25,
        "y": 0.0,
        "z": 0.0
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
        "node": 3,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 3,
        "pin": "first"
      },
      "to": {
        "node": 4,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 2,
        "pin": "down"
      },
      "to": {
        "node": 4,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 4,
        "pin": "true"
      },
      "to": {
        "node": 5,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 3,
        "pin": "second"
      },
      "to": {
        "node": 7,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 7,
        "pin": "first"
      },
      "to": {
        "node": 8,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 6,
        "pin": "pressed"
      },
      "to": {
        "node": 8,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 8,
        "pin": "true"
      },
      "to": {
        "node": 9,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 7,
        "pin": "second"
      },
      "to": {
        "node": 11,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 11,
        "pin": "first"
      },
      "to": {
        "node": 12,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 10,
        "pin": "released"
      },
      "to": {
        "node": 12,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 12,
        "pin": "true"
      },
      "to": {
        "node": 13,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 11,
        "pin": "second"
      },
      "to": {
        "node": 15,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 15,
        "pin": "first"
      },
      "to": {
        "node": 16,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 14,
        "pin": "pressed"
      },
      "to": {
        "node": 16,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 16,
        "pin": "true"
      },
      "to": {
        "node": 17,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 15,
        "pin": "second"
      },
      "to": {
        "node": 19,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 19,
        "pin": "first"
      },
      "to": {
        "node": 20,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 18,
        "pin": "released"
      },
      "to": {
        "node": 20,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 20,
        "pin": "true"
      },
      "to": {
        "node": 21,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 19,
        "pin": "second"
      },
      "to": {
        "node": 23,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 23,
        "pin": "first"
      },
      "to": {
        "node": 24,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 22,
        "pin": "down"
      },
      "to": {
        "node": 24,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 24,
        "pin": "true"
      },
      "to": {
        "node": 25,
        "pin": "in"
      }
    }
  ]
}
]==]

local vars = {}
local state = {}

function on_update()
    -- On Update [1]
    -- Branch [4]
    if vs.input("mouse", "down", "Left") then
        -- Translate [5]
        vs.translate(entity, 0.5, 0, 0)
    end
    -- Branch [8]
    if vs.input("mouse", "pressed", "Right") then
        -- Translate [9]
        vs.translate(entity, 0, 1, 0)
    end
    -- Branch [12]
    if vs.input("mouse", "released", "Left") then
        -- Translate [13]
        vs.translate(entity, 0, 0, 1)
    end
    -- Branch [16]
    if vs.input("key", "pressed", "KeyW") then
        -- Set Scale [17]
        vs.set_scale(entity, 2, 1, 0.5)
    end
    -- Branch [20]
    if vs.input("key", "released", "Space") then
        -- Rotate [21]
        vs.rotate(entity, 0, 10, 0)
    end
    -- Branch [24]
    if vs.input("key", "down", "KeyR") then
        -- Translate [25]
        vs.translate(entity, -0.25, 0, 0)
    end
end
