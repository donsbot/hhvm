<?hh // partial
// generated by idl-to-hni.php

/** This function returns an array with the current available SPL classes.
 * @return array - Returns an array containing the currently available SPL
 * classes.
 */
function spl_classes(): darray {
  return darray[
    ArrayIterator::class => ArrayIterator::class,
    BadFunctionCallException::class => BadFunctionCallException::class,
    BadMethodCallException::class => BadMethodCallException::class,
    Countable::class => Countable::class,
    DirectoryIterator::class => DirectoryIterator::class,
    DomainException::class => DomainException::class,
    EmptyIterator::class => EmptyIterator::class,
    FilesystemIterator::class => FilesystemIterator::class,
    FilterIterator::class => FilterIterator::class,
    GlobIterator::class => GlobIterator::class,
    InfiniteIterator::class => InfiniteIterator::class,
    InvalidArgumentException::class => InvalidArgumentException::class,
    IteratorIterator::class => IteratorIterator::class,
    LengthException::class => LengthException::class,
    LogicException::class => LogicException::class,
    NoRewindIterator::class => NoRewindIterator::class,
    OuterIterator::class => OuterIterator::class,
    OutOfBoundsException::class => OutOfBoundsException::class,
    OutOfRangeException::class => OutOfRangeException::class,
    OverflowException::class => OverflowException::class,
    RangeException::class => RangeException::class,
    RecursiveDirectoryIterator::class => RecursiveDirectoryIterator::class,
    RecursiveFilterIterator::class => RecursiveFilterIterator::class,
    RecursiveIterator::class => RecursiveIterator::class,
    RecursiveIteratorIterator::class => RecursiveIteratorIterator::class,
    RecursiveRegexIterator::class => RecursiveRegexIterator::class,
    RegexIterator::class => RegexIterator::class,
    RuntimeException::class => RuntimeException::class,
    SeekableIterator::class => SeekableIterator::class,
    SplDoublyLinkedList::class => SplDoublyLinkedList::class,
    SplFileInfo::class => SplFileInfo::class,
    SplFileObject::class => SplFileObject::class,
    SplHeap::class => SplHeap::class,
    SplMinHeap::class => SplMinHeap::class,
    SplMaxHeap::class => SplMaxHeap::class,
    SplObserver::class => SplObserver::class,
    SplPriorityQueue::class => SplPriorityQueue::class,
    SplQueue::class => SplQueue::class,
    SplStack::class => SplStack::class,
    SplSubject::class => SplSubject::class,
    SplTempFileObject::class => SplTempFileObject::class,
    UnderflowException::class => UnderflowException::class,
    UnexpectedValueException::class => UnexpectedValueException::class,
  ];
}

/** This function returns a unique identifier for the object. This id can be
 * used as a hash key for storing objects or for identifying an object.
 * @param object $obj - Any object.
 * @return string - A string that is unique for each currently existing object
 * and is always the same for each object.
 */
<<__Native>>
function spl_object_hash(object $obj)[]: string;

/** This function returns low level raw pointer the object. Used by closure and
 * internal purposes.
 * @param object $obj - Any object.
 * @return int - Low level ObjectData pointer.
 */
<<__Native("NoInjection")>>
function hphp_object_pointer(object $obj): int;

/** This function returns this object if present, or NULL.
 * @return mixed - This object.
 */
<<__Native("NoInjection")>>
function hphp_get_this(): mixed;

/** This function returns an array with the names of the interfaces that the
 * given class and its parents implement.
 * @param mixed $obj - An object (class instance) or a string (class name).
 * @param bool $autoload - Whether to allow this function to load the class
 * automatically.
 * @return mixed - An array on success, or FALSE on error.
 */
<<__Native>>
function class_implements(mixed $obj,
                          bool $autoload = true)[]: mixed;

/** This function returns an array with the name of the parent classes of the
 * given class.
 * @param mixed $obj - An object (class instance) or a string (class name).
 * @param bool $autoload - Whether to allow this function to load the class
 * automatically.
 * @return mixed - An array on success, or FALSE on error.
 */
<<__Native>>
function class_parents(mixed $obj,
                       bool $autoload = true)[]: mixed;

/** This function returns an array with the names of the traits that the given
 * class uses.
 * @param mixed $obj - An object (class instance) or a string (class name).
 * @param bool $autoload - Whether to allow this function to load the class
 * automatically.
 * @return mixed - An array on success, or FALSE on error.
 */
<<__Native>>
function class_uses(mixed $obj,
                    bool $autoload = true)[]: mixed;

/** Calls a function for every element in an iterator.
 * @param mixed $obj - The class to iterate over.
 * @param mixed $func - The callback function to call on every element. The
 * function must return TRUE in order to continue iterating over the iterator.
 * @param array $params - Arguments to pass to the callback function.
 * @return mixed - Returns the iteration count.
 */
function iterator_apply(mixed $obj, mixed $func, varray $params = varray[]): mixed {
  if (!is_object($obj) || !($obj is \HH\Traversable)) {
    trigger_error("Argument must implement interface Traversable", E_RECOVERABLE_ERROR);
    return 0;
  }
  $count = 0;
  foreach ($obj as $v) {
    if ($func(...$params) !== true) {
      break;
    }
    ++$count;
  }
  return $count;
}

/** Count the elements in an iterator.
 * @param mixed $obj - The iterator being counted.
 * @return mixed - The number of elements in iterator.
 */
function iterator_count(mixed $obj): mixed {
  if (!is_object($obj) || !($obj is \HH\Traversable)) {
    trigger_error("Argument must implement interface Traversable", E_RECOVERABLE_ERROR);
    return 0;
  }
  $count = 0;
  foreach ($obj as $_) {
    ++$count;
  }
  return $count;
}

/** Copy the elements of an iterator into an array.
 * @param mixed $obj - The iterator being copied.
 * @param bool $use_keys - Whether to use the iterator element keys as index.
 * @return mixed - An array containing the elements of the iterator.
 */
function iterator_to_array(mixed $obj, bool $use_keys = true): darray {
  if (!is_object($obj) || !($obj is \HH\Traversable)) {
    trigger_error("Argument must implement interface Traversable", E_RECOVERABLE_ERROR);
    return 0;
  }
  $ret = darray[];
  if ($use_keys) {
    foreach ($obj as $k => $v) {
      $ret[$k] = $v;
    }
  } else {
    foreach ($obj as $v) {
      $ret[] = $v;
    }
  }
  return $ret;
}
