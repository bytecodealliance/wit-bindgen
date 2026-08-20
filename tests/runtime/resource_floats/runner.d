import wit.test.resource_floats.runner;

import wit.test.resource_floats.runner.imports.exports : Float2 = Float;

import wit.common;

@witExport("$root", "run")
void run() {
    auto float1 = Float.makeNew(42);
    scope(exit) float1.witDrop;

    auto float2 = Float.makeNew(55);
    scope(exit) float2.witDrop;

    auto float3 = add(float1, float2);
    scope(exit) float3.witDrop;
    assert(float3.get == 114.0);

    auto float4 = Float2.makeNew(22);
    scope(exit) float4.witDrop;
    assert(float4.get == 22.0 + 1.0 + 2.0 + 4.0 + 3.0);

    auto res = Float2.add(float4, 7.0);
    scope(exit) res.witDrop;
    float4 = Float2.init; // consumed
    assert(res.get == 59.0);
}

alias Exports = wit.test.resource_floats.runner.Exports!(
    run
);
