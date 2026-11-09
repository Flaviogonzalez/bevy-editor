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
      "kind": "event.start",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 2,
      "kind": "action.print",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": "Hello"
      }
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
      "kind": "event.update",
      "x": 0.0,
      "y": 0.0
    },
    {
      "id": 5,
      "kind": "action.print",
      "x": 0.0,
      "y": 0.0,
      "values": {
        "value": "Hello"
      }
    },
    {
      "id": 6,
      "kind": "component.get",
      "x": 0.0,
      "y": 0.0,
      "field": "Wobble.amount"
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
        "pin": "value"
      },
      "to": {
        "node": 2,
        "pin": "value"
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
        "node": 5,
        "pin": "value"
      }
    }
  ]
}
]==]

local vars = {}
local state = {}

function on_script_loaded()
    -- On Start [1]
    -- Print [2]
    do
        local t0 = node.text.from_number(vs.get(entity, "Stats", "power"))
        print(t0)
    end
end

function on_update()
    -- On Update [4]
    -- Print [5]
    do
        local t1 = node.text.from_number(vs.get(entity, "Wobble", "amount"))
        print(t1)
    end
end
