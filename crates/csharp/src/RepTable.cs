/**
 * This class is used to assign a unique integer identifier to instances of
 * exported resources, which the host will use as its "core representation" per
 * https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md#definition-types.
 * The identifier may be used to retrieve the corresponding instance e.g. when
 * lifting a handle as part of the canonical ABI implementation.
 */
internal class RepTable<T> {
    private List<object> list = new List<object>();
    private int? firstVacant = null;
    
    private class Vacant {
        internal int? next;

        internal Vacant(int? next) {
            this.next = next;
        }
    }

    internal int Add(T v) {
        int rep;
        if (firstVacant.HasValue) {
            rep = firstVacant.Value;
            firstVacant = ((Vacant) list[rep]).next;
            list[rep] = v!;
        } else {
            rep = list.Count;
            list.Add(v!);
        }
        return rep;
    }

    internal T Get(nint rep) {
        if (list[(int)rep] is Vacant) {
            throw new global::System.ArgumentException("invalid rep");
        }
        return (T) list[(int)rep];
    }

    internal T Remove(nint rep) {
        var val = Get(rep);
        list[(int)rep] = new Vacant(firstVacant);
        firstVacant = (int)rep;
        return (T) val;
    }
}
