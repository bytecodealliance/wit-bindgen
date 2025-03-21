using TestWorld.wit.exports.test.resourceBorrowInRecord;

public class ToTestImpl : IToTest {
    public class Thing : IToTest.Thing, IToTest.IThing {
        public string val;

        public Thing(string v) {
            this.val = v + " new";
        }

        public Thing(Thing other) {
            this.val = other.val + " test";
        }

        public string Get() {
            return val + " get";
        }
    }

    public static List<IToTest.Thing> Test(List<IToTest.Foo> v) {
        var myResult = new List<IToTest.Thing>();
        foreach (var foo in v)
        {
            myResult.Add(new Thing((Thing) foo.thing));
        }
        return myResult;
    }
}

