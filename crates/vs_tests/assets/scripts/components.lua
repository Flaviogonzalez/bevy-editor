-- A visual script. bevy-editor generated this file from the graph below.
-- Edit the graph in the editor: changes to the code are replaced on save.
--[==[bevy-visual-script
{
  "version": 1,
  "next_id": 30,
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
      "kind": "time.elapsed",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 3,
      "kind": "math.sin",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0
      }
    },
    {
      "id": 4,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Wobble.amount"
    },
    {
      "id": 5,
      "kind": "math.mul",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.25
      }
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
      "field": "Wobble.amount"
    },
    {
      "id": 8,
      "kind": "vector.new",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 1.0,
        "z": 0.5
      }
    },
    {
      "id": 9,
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
      "field": "Wobble.axis"
    },
    {
      "id": 10,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Transform.translation"
    },
    {
      "id": 11,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Wobble.axis"
    },
    {
      "id": 12,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Wobble.amount"
    },
    {
      "id": 13,
      "kind": "math.mul",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.1
      }
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
      "id": 16,
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
      "id": 17,
      "kind": "color.srgba",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "alpha": 1.0,
        "blue": 0.25,
        "green": 0.5,
        "red": 0.0
      }
    },
    {
      "id": 18,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": {
          "rgba": [
            1.0,
            1.0,
            1.0,
            1.0
          ]
        }
      },
      "field": "Wobble.tint"
    },
    {
      "id": 19,
      "kind": "color.red",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "color": {
          "rgba": [
            1.0,
            1.0,
            1.0,
            1.0
          ]
        }
      }
    },
    {
      "id": 20,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Wobble.tint"
    },
    {
      "id": 21,
      "kind": "transform.translate",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 0.0
      }
    },
    {
      "id": 22,
      "kind": "logic.gt",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "a": 0.0,
        "b": 0.0
      }
    },
    {
      "id": 23,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Wobble.amount"
    },
    {
      "id": 24,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": false
      },
      "field": "Wobble.active"
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
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Wobble.active"
    },
    {
      "id": 27,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": [
          1.5,
          1.0,
          1.25
        ]
      },
      "field": "Transform.scale"
    },
    {
      "id": 28,
      "kind": "component.set",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": {
          "euler": [
            0.0,
            0.0,
            0.0
          ]
        }
      },
      "field": "Transform.rotation"
    },
    {
      "id": 29,
      "kind": "rotation.from_euler",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "x": 0.0,
        "y": 0.0,
        "z": 0.0
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
        "node": 3,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 3,
        "pin": "result"
      },
      "to": {
        "node": 5,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 4,
        "pin": "value"
      },
      "to": {
        "node": 6,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 5,
        "pin": "result"
      },
      "to": {
        "node": 6,
        "pin": "b"
      }
    },
    {
      "from": {
        "node": 1,
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
        "node": 3,
        "pin": "result"
      },
      "to": {
        "node": 8,
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 7,
        "pin": "then"
      },
      "to": {
        "node": 9,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 8,
        "pin": "result"
      },
      "to": {
        "node": 9,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 12,
        "pin": "value"
      },
      "to": {
        "node": 13,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 11,
        "pin": "value"
      },
      "to": {
        "node": 14,
        "pin": "v"
      }
    },
    {
      "from": {
        "node": 13,
        "pin": "result"
      },
      "to": {
        "node": 14,
        "pin": "factor"
      }
    },
    {
      "from": {
        "node": 10,
        "pin": "value"
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
        "node": 9,
        "pin": "then"
      },
      "to": {
        "node": 16,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 15,
        "pin": "result"
      },
      "to": {
        "node": 16,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 3,
        "pin": "result"
      },
      "to": {
        "node": 17,
        "pin": "red"
      }
    },
    {
      "from": {
        "node": 16,
        "pin": "then"
      },
      "to": {
        "node": 18,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 17,
        "pin": "result"
      },
      "to": {
        "node": 18,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 20,
        "pin": "value"
      },
      "to": {
        "node": 19,
        "pin": "color"
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
        "pin": "x"
      }
    },
    {
      "from": {
        "node": 23,
        "pin": "value"
      },
      "to": {
        "node": 22,
        "pin": "a"
      }
    },
    {
      "from": {
        "node": 21,
        "pin": "then"
      },
      "to": {
        "node": 24,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 22,
        "pin": "result"
      },
      "to": {
        "node": 24,
        "pin": "value"
      }
    },
    {
      "from": {
        "node": 24,
        "pin": "then"
      },
      "to": {
        "node": 25,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 26,
        "pin": "value"
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
        "node": 27,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 3,
        "pin": "result"
      },
      "to": {
        "node": 29,
        "pin": "y"
      }
    },
    {
      "from": {
        "node": 25,
        "pin": "false"
      },
      "to": {
        "node": 28,
        "pin": "in"
      }
    },
    {
      "from": {
        "node": 29,
        "pin": "result"
      },
      "to": {
        "node": 28,
        "pin": "value"
      }
    }
  ]
}
]==]

local vars = {}
local state = {}

function on_update()
    -- On Update [1]
    -- Set Wobble.amount [7]
    do
        local t0 = node.math.sin(vs.time())
        local t1 = node.math.mul(t0, 0.25)
        local t2 = node.math.add(vs.get(entity, "Wobble", "amount"), t1)
        vs.set(entity, "Wobble", "amount", t2)
    end
    -- Set Wobble.axis [9]
    do
        local t3 = node.math.sin(vs.time())
        local t4 = node.vector.new(t3, 1, 0.5)
        vs.set(entity, "Wobble", "axis", t4)
    end
    -- Set Transform.translation [16]
    do
        local t5 = node.math.mul(vs.get(entity, "Wobble", "amount"), 0.1)
        local t6 = node.vector.scale(vs.get(entity, "Wobble", "axis"), t5)
        local t7 = node.vector.add(vs.get(entity, "Transform", "translation"), t6)
        vs.set(entity, "Transform", "translation", t7)
    end
    -- Set Wobble.tint [18]
    do
        local t8 = node.math.sin(vs.time())
        local t9 = node.color.srgba(t8, 0.5, 0.25, 1)
        vs.set(entity, "Wobble", "tint", t9)
    end
    -- Translate [21]
    do
        local t10 = node.color.red(vs.get(entity, "Wobble", "tint"))
        vs.translate(entity, t10, 0, 0)
    end
    -- Set Wobble.active [24]
    do
        local t11 = node.logic.gt(vs.get(entity, "Wobble", "amount"), 0)
        vs.set(entity, "Wobble", "active", t11)
    end
    -- Branch [25]
    if vs.get(entity, "Wobble", "active") then
        -- Set Transform.scale [27]
        vs.set(entity, "Transform", "scale", vs.vec3(1.5, 1, 1.25))
    else
        -- Set Transform.rotation [28]
        do
            local t12 = node.math.sin(vs.time())
            local t13 = node.rotation.from_euler(0, t12, 0)
            vs.set(entity, "Transform", "rotation", t13)
        end
    end
end
