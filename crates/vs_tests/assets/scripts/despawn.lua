-- A visual script. bevy-editor generated this file from the graph below.
-- Edit the graph in the editor: changes to the code are replaced on save.
--[==[bevy-visual-script
{
  "version": 1,
  "next_id": 7,
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
      "kind": "input.key_released",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "key": "KeyR"
      }
    },
    {
      "id": 3,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 4,
      "kind": "entity.despawn",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 5,
      "kind": "transform.translate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.125,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 6,
      "kind": "transform.translate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 0.0625,
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
        "node": 2,
        "pin": "released"
      },
      "to": {
        "node": 3,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 3,
        "pin": "true"
      },
      "to": {
        "node": 4,
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
        "node": 3,
        "pin": "false"
      },
      "to": {
        "node": 6,
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
    -- Branch [3]
    if vs.input("key", "released", "KeyR") then
        -- Despawn [4]
        vs.despawn(entity)
        -- Translate [5]
        vs.translate(entity, 0.125, 0, 0)
    else
        -- Translate [6]
        vs.translate(entity, 0, 0.0625, 0)
    end
end
