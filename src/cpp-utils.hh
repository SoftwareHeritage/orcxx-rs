#include <memory>

namespace orcxx_rs {

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
    get_numElements(T &obj)
    {
      return obj.numElements;
    }
}
