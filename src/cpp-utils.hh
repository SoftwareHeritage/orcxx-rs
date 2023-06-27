#include <memory>

#include <orc/MemoryPool.hh>
#include <orc/Vector.hh>


#define getter(name) \
    template<typename T, typename Ret> \
    Ret \
    get_## name(T &obj) \
    { \
      return obj.name; \
    }

namespace orcxx_rs {

    namespace utils {
        // Constructs a C++ object using this trick:
        // https://github.com/dtolnay/cxx/issues/280#issuecomment-1344153115
        template<typename T, typename... Args>
        std::unique_ptr<T>
        construct(Args... args)
        {
          return std::make_unique<T>(args...);
        }

        template<typename T, typename Ret>
        Ret
        try_into(T obj) {
          return dynamic_cast<Ret>(obj);
        }

        template<typename T, typename Ret>
        std::unique_ptr<Ret>
        ptr_try_into(std::unique_ptr<T> obj) {
          std::unique_ptr<Ret> p(dynamic_cast<Ret*>(obj.get()));
          return p;
        }

        template<typename T, typename Ret>
        Ret
        into(T obj)
        {
          return obj;
        }
    }

    namespace accessors {
        getter(numElements);
        getter(length);
        getter(data);
        getter(fields);
    }

    typedef orc::DataBuffer<char*> StringDataBuffer;
    typedef orc::DataBuffer<int64_t> Int64DataBuffer;
    typedef orc::ColumnVectorBatch* ColumnVectorBatchPtr;
}

