-- A visual script. bevy-editor generated this file from the graph below.
-- Edit the graph in the editor: changes to the code are replaced on save.
--[==[bevy-visual-script
{
  "version": 1,
  "next_id": 22,
  "variables": [],
  "nodes": [
    {
      "id": 1,
      "kind": "define.component",
      "x": 0.0,
      "y": 0.0,
      "component": {
        "name": "Charge",
        "fields": [
          {
            "name": "level",
            "value": 0.0
          }
        ]
      }
    },
    {
      "id": 2,
      "kind": "event.update",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 3,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Stats.power"
    },
    {
      "id": 4,
      "kind": "math.mul",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 5,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Charge.level"
    },
    {
      "id": 6,
      "kind": "math.add",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 7,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": 0.0
      },
      "field": "Charge.level"
    },
    {
      "id": 8,
      "kind": "logic.lt",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 3.0
      }
    },
    {
      "id": 9,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Charge.level"
    },
    {
      "id": 10,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": false
      },
      "field": "Stats.on"
    },
    {
      "id": 11,
      "kind": "flow.branch",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "condition": false
      }
    },
    {
      "id": 12,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Stats.on"
    },
    {
      "id": 13,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Stats.drift"
    },
    {
      "id": 14,
      "kind": "vector.scale",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "factor": 0.0,
        "v": [
          0.0,
          0.0,
          0.0
        ]
      }
    },
    {
      "id": 15,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Transform.translation"
    },
    {
      "id": 16,
      "kind": "vector.add",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": [
          0.0,
          0.0,
          0.0
        ],
        "b": [
          0.0,
          0.0,
          0.0
        ]
      }
    },
    {
      "id": 17,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": [
          0.0,
          0.0,
          0.0
        ]
      },
      "field": "Transform.translation"
    },
    {
      "id": 18,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": 0.0
      },
      "field": "Charge.level"
    },
    {
      "id": 19,
      "kind": "math.add",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.5
      }
    },
    {
      "id": 20,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Stats.power"
    },
    {
      "id": 21,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": 0.0
      },
      "field": "Stats.power"
    }
  ],
  "links": [
    {
      "from": {
        "node": 2,
        "pin": "delta"
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
        "node": 5,
        "pin": "value"
      },
      "to": {
        "node": 6,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 4,
        "pin": "result"
      },
      "to": {
        "node": 6,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 2,
        "pin": "then"
      },
      "to": {
        "node": 7,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 6,
        "pin": "result"
      },
      "to": {
        "node": 7,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 9,
        "pin": "value"
      },
      "to": {
        "node": 8,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 7,
        "pin": "then"
      },
      "to": {
        "node": 10,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 8,
        "pin": "result"
      },
      "to": {
        "node": 10,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 10,
        "pin": "then"
      },
      "to": {
        "node": 11,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 12,
        "pin": "value"
      },
      "to": {
        "node": 11,
        "pin": "condition"
      }
    },
    {
      "from": {
        "node": 13,
        "pin": "value"
      },
      "to": {
        "node": 14,
        "pin": "v"
      }
    },
    {
      "from": {
        "node": 2,
        "pin": "delta"
      },
      "to": {
        "node": 14,
        "pin": "factor"
      }
    },
    {
      "from": {
        "node": 15,
        "pin": "value"
      },
      "to": {
        "node": 16,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 14,
        "pin": "result"
      },
      "to": {
        "node": 16,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 11,
        "pin": "true"
      },
      "to": {
        "node": 17,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 16,
        "pin": "result"
      },
      "to": {
        "node": 17,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 11,
        "pin": "false"
      },
      "to": {
        "node": 18,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 20,
        "pin": "value"
      },
      "to": {
        "node": 19,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 18,
        "pin": "then"
      },
      "to": {
        "node": 21,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 19,
        "pin": "result"
      },
      "to": {
        "node": 21,
        "pin": "value"
      }
    }
  ]
}
]==]

local vars = {}
local state = {}

function on_update()
    -- On Update [2]
    -- Set Charge.level [7]
    do
        local t0 = node.math.mul(vs.delta(), vs.get(entity, "Stats", "power"))
        local t1 = node.math.add(vs.get(entity, "Charge", "level"), t0)
        vs.set(entity, "Charge", "level", t1)
    end
    -- Set Stats.on [10]
    do
        local t2 = node.logic.lt(vs.get(entity, "Charge", "level"), 3)
        vs.set(entity, "Stats", "on", t2)
    end
    -- Branch [11]
    if vs.get(entity, "Stats", "on") then
        -- Set Transform.translation [17]
        do
            local t3 = node.vector.scale(vs.get(entity, "Stats", "drift"), vs.delta())
            local t4 = node.vector.add(vs.get(entity, "Transform", "translation"), t3)
            vs.set(entity, "Transform", "translation", t4)
        end
    else
        -- Set Charge.level [18]
        vs.set(entity, "Charge", "level", 0)
        -- Set Stats.power [21]
        do
            local t5 = node.math.add(vs.get(entity, "Stats", "power"), 0.5)
            vs.set(entity, "Stats", "power", t5)
        end
    end
end
